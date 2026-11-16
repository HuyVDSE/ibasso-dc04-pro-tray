use hidapi::HidApi;
use ibasso_dc04_pro::dc04::{Device, Error, MAX_TARGET, MIN_TARGET, VolumeReading};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(350);
const RETRY: Duration = Duration::from_millis(650);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Reading,
    Offline,
    Multiple,
    Unavailable,
    Sync,
    Read(VolumeReading),
}

#[derive(Clone, Copy)]
pub struct Snapshot {
    pub epoch: u64,
    pub status: Status,
    pub requested: Option<u8>,
}

impl Snapshot {
    pub fn adjustable(self) -> bool {
        matches!(self.status, Status::Read(reading) if reading.adjustable())
    }
}

struct State {
    snapshot: Snapshot,
    last_error: Option<String>,
    pending: Option<u8>,
    visible: bool,
    refresh: bool,
    stop: bool,
}

impl State {
    fn new() -> Self {
        Self {
            snapshot: Snapshot {
                epoch: 0,
                status: Status::Reading,
                requested: None,
            },
            last_error: None,
            pending: None,
            visible: false,
            refresh: false,
            stop: false,
        }
    }

    fn invalidate(&mut self, status: Status) {
        self.snapshot.epoch += 1;
        self.snapshot.status = status;
        self.snapshot.requested = None;
        self.pending = None;
    }

    fn submit(&mut self, epoch: u64, step: u8) -> bool {
        if self.stop
            || self.snapshot.epoch != epoch
            || !self.snapshot.adjustable()
            || !(MIN_TARGET..=MAX_TARGET).contains(&step)
        {
            return false;
        }
        self.pending = Some(step);
        self.snapshot.requested = Some(step);
        true
    }

    fn confirm(&mut self, reading: VolumeReading) {
        let was_adjustable = self.snapshot.adjustable();
        self.snapshot.status = Status::Read(reading);
        if was_adjustable && !reading.adjustable() {
            self.invalidate(Status::Read(reading));
        } else if self.pending.is_none() {
            self.snapshot.requested = None;
        }
    }
}

pub struct Worker {
    shared: Arc<(Mutex<State>, Condvar)>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn start(notify: impl Fn() + Send + 'static) -> std::io::Result<Self> {
        let shared = Arc::new((Mutex::new(State::new()), Condvar::new()));
        let for_thread = shared.clone();
        let thread = thread::Builder::new()
            .name("dc04-hid".into())
            .spawn(move || run(&for_thread, notify))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.shared.0.lock().unwrap().snapshot
    }

    pub fn last_error(&self) -> Option<String> {
        self.shared.0.lock().unwrap().last_error.clone()
    }

    pub fn rescan(&self) {
        self.shared.0.lock().unwrap().invalidate(Status::Reading);
        self.shared.1.notify_one();
    }

    pub fn visible(&self, visible: bool) {
        let mut state = self.shared.0.lock().unwrap();
        state.visible = visible;
        state.refresh = visible;
        self.shared.1.notify_one();
    }

    pub fn submit(&self, epoch: u64, step: u8) -> bool {
        let accepted = self.shared.0.lock().unwrap().submit(epoch, step);
        if accepted {
            self.shared.1.notify_one();
        }
        accepted
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.shared.0.lock().unwrap().stop = true;
        self.shared.1.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

enum Work {
    Open,
    Read,
    Write(u8),
}

fn run((lock, wake): &(Mutex<State>, Condvar), notify: impl Fn()) {
    let mut device: Option<Device> = None;
    let mut epoch = 0;
    let mut due = Instant::now();
    loop {
        let work = {
            let mut state = lock.lock().unwrap();
            loop {
                if state.stop {
                    return;
                }
                if epoch != state.snapshot.epoch {
                    // An explicit rescan invalidates the handle even if the same path reappears.
                    epoch = state.snapshot.epoch;
                    break Work::Open;
                }
                if let Some(step) = state.pending.take() {
                    break Work::Write(step);
                }
                let now = Instant::now();
                if device.is_none() && now >= due {
                    break Work::Open;
                }
                if device.is_some() && state.visible && (state.refresh || now >= due) {
                    state.refresh = false;
                    break Work::Read;
                }
                state = if device.is_none() || state.visible {
                    wake.wait_timeout(state, due.saturating_duration_since(now))
                        .unwrap()
                        .0
                } else {
                    wake.wait(state).unwrap()
                };
            }
        };

        let opening = matches!(work, Work::Open);
        let active = || {
            let state = lock.lock().unwrap();
            !state.stop && state.snapshot.epoch == epoch
        };
        let result = match work {
            Work::Open => {
                device = None;
                HidApi::new()
                    .map_err(|source| Error::Hid {
                        operation: "enumerating HID devices",
                        source,
                    })
                    .and_then(|api| Device::open(&api))
                    .and_then(|opened| {
                        let reading = opened.volume_reading()?;
                        device = Some(opened);
                        Ok(reading)
                    })
            }
            Work::Read => device.as_ref().unwrap().volume_reading(),
            Work::Write(step) => device.as_ref().unwrap().set_volume(step, active),
        };

        let close = result.is_err();
        {
            let mut state = lock.lock().unwrap();
            if state.stop || state.snapshot.epoch != epoch {
                continue;
            }
            match result {
                Ok(reading) => {
                    state.last_error = None;
                    state.confirm(reading);
                    epoch = state.snapshot.epoch;
                    due = Instant::now() + POLL;
                    if opening {
                        state.refresh = false;
                    }
                }
                Err(error) => {
                    state.last_error = Some(error.to_string());
                    let status = match error {
                        Error::NoDevice => Status::Offline,
                        Error::MultipleDevices => Status::Multiple,
                        _ if !opening || state.snapshot.status == Status::Sync => Status::Sync,
                        _ => Status::Unavailable,
                    };
                    state.invalidate(status);
                    epoch = state.snapshot.epoch;
                    // Open a fresh handle after a quiet period so a late reply to a
                    // failed transaction cannot be reused as its own readback.
                    due = Instant::now() + RETRY;
                }
            }
        }
        notify();
        if close {
            device = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready(state: &mut State) {
        state.confirm(VolumeReading {
            reg_10: 0x5c,
            reg_11: 0x5c,
        });
    }

    #[test]
    fn only_the_latest_target_survives_while_a_write_is_in_flight() {
        let mut state = State::new();
        ready(&mut state);
        assert!(state.submit(0, 29));
        assert_eq!(state.pending.take(), Some(29));
        assert!(state.submit(0, 30));
        assert!(state.submit(0, 31));
        ready(&mut state);
        assert_eq!(state.snapshot.requested, Some(31));
        assert_eq!(state.pending.take(), Some(31));
        ready(&mut state);
        assert_eq!(state.snapshot.requested, None);
    }

    #[test]
    fn neither_disconnect_nor_resync_can_replay_a_pending_or_late_target() {
        for status in [Status::Reading, Status::Sync] {
            let mut state = State::new();
            ready(&mut state);
            assert!(state.submit(0, 29));
            state.invalidate(status);
            assert_eq!(state.pending, None);
            assert!(!state.submit(1, 30));
            ready(&mut state);
            assert!(!state.submit(0, 30));
            assert!(state.submit(1, 30));
        }
    }

    #[test]
    fn partial_pair_and_edges_stay_visible_without_accepting_writes() {
        for (reg_10, reg_11) in [(0x5c, 0x5d), (0, 0), (255, 255)] {
            let mut state = State::new();
            ready(&mut state);
            assert!(state.submit(0, 29));
            let reading = VolumeReading { reg_10, reg_11 };
            state.confirm(reading);
            assert_eq!(state.snapshot.status, Status::Read(reading));
            assert_eq!(state.pending, None);
            assert!(!state.submit(state.snapshot.epoch, 28));
        }
    }
}
