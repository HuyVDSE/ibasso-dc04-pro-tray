use hidapi::HidApi;
use ibasso_dc04_pro::dc04::{Device, Error, GainLevel, MAX_TARGET, MIN_TARGET, VolumeReading};
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

pub struct GainSnapshot {
    pub epoch: u64,
    pub status: Status,
    pub requested: Option<GainLevel>,
    pub last_sent: Option<GainLevel>,
    pub error: Option<String>,
}

impl GainSnapshot {
    pub fn can_set_gain(&self) -> bool {
        matches!(self.status, Status::Read(_)) && self.requested.is_none()
    }
}

#[derive(Default)]
struct GainState {
    pending: Option<GainLevel>,
    requested: Option<GainLevel>,
    last_sent: Option<GainLevel>,
    error: Option<String>,
}

struct State {
    snapshot: Snapshot,
    gain: GainState,
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
            gain: GainState::default(),
            last_error: None,
            pending: None,
            visible: false,
            refresh: false,
            stop: false,
        }
    }

    fn invalidate(&mut self, status: Status) {
        self.invalidate_volume(status);
        self.gain.pending = None;
        self.gain.requested = None;
        self.gain.last_sent = None;
        // Keep the previous write error available after reconnect or a successful read.
    }

    fn invalidate_volume(&mut self, status: Status) {
        self.snapshot.epoch += 1;
        self.snapshot.status = status;
        self.snapshot.requested = None;
        self.pending = None;
    }

    fn gain_snapshot(&self) -> GainSnapshot {
        GainSnapshot {
            epoch: self.snapshot.epoch,
            status: self.snapshot.status,
            requested: self.gain.requested,
            last_sent: self.gain.last_sent,
            error: self.gain.error.clone(),
        }
    }

    fn submit_gain(&mut self, epoch: u64, level: GainLevel) -> bool {
        if self.stop
            || self.snapshot.epoch != epoch
            || !matches!(self.snapshot.status, Status::Read(_))
            || self.gain.requested.is_some()
        {
            return false;
        }
        self.gain.pending = Some(level);
        self.gain.requested = Some(level);
        self.gain.error = None;
        true
    }

    fn active(&self, epoch: u64) -> bool {
        !self.stop && self.snapshot.epoch == epoch
    }

    fn complete(&mut self, epoch: u64, work: Work, result: Result<Completed, Error>) -> bool {
        if !self.active(epoch) {
            return false;
        }
        match result {
            Ok(Completed::Volume(reading)) => {
                self.last_error = None;
                self.confirm(reading);
                if matches!(work, Work::Open) {
                    self.refresh = false;
                }
            }
            Ok(Completed::Gain(level)) => {
                self.gain.requested = None;
                self.gain.last_sent = Some(level);
            }
            Err(error) => {
                let detail = error.to_string();
                if matches!(work, Work::WriteGain(_)) {
                    self.gain.error = Some(detail.clone());
                }
                self.last_error = Some(detail);
                let status = match error {
                    Error::NoDevice => Status::Offline,
                    Error::MultipleDevices => Status::Multiple,
                    _ if !matches!(work, Work::Open) || self.snapshot.status == Status::Sync => {
                        Status::Sync
                    }
                    _ => Status::Unavailable,
                };
                self.invalidate(status);
            }
        }
        true
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
            // A balance/limit change invalidates slider input on this same HID handle.
            self.invalidate_volume(Status::Read(reading));
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

    pub fn gain_snapshot(&self) -> GainSnapshot {
        self.shared.0.lock().unwrap().gain_snapshot()
    }

    pub fn submit_gain(&self, epoch: u64, level: GainLevel) -> bool {
        let accepted = self.shared.0.lock().unwrap().submit_gain(epoch, level);
        if accepted {
            self.shared.1.notify_one();
        }
        accepted
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

#[derive(Clone, Copy)]
enum Work {
    Open,
    Read,
    Write(u8),
    WriteGain(GainLevel),
}

enum Completed {
    Volume(VolumeReading),
    Gain(GainLevel),
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
                if let Some(level) = state.gain.pending.take() {
                    break Work::WriteGain(level);
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

        let active = || lock.lock().unwrap().active(epoch);
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
                        Ok(Completed::Volume(reading))
                    })
            }
            Work::Read => device
                .as_ref()
                .unwrap()
                .volume_reading()
                .map(Completed::Volume),
            Work::Write(step) => device
                .as_ref()
                .unwrap()
                .set_volume(step, active)
                .map(Completed::Volume),
            Work::WriteGain(level) => device
                .as_ref()
                .unwrap()
                .set_gain(level, active, |_, _| {})
                .map(|()| Completed::Gain(level)),
        };

        let close = result.is_err();
        {
            let mut state = lock.lock().unwrap();
            if !state.complete(epoch, work, result) {
                continue;
            }
            epoch = state.snapshot.epoch;
            // A failed transaction gets a fresh handle after a quiet period.
            due = Instant::now() + if close { RETRY } else { POLL };
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
    fn gain_lifecycle_tracks_requests_and_only_records_completed_writes() {
        let mut state = State::new();
        assert!(!state.gain_snapshot().can_set_gain());
        assert!(!state.submit_gain(0, GainLevel::High));
        ready(&mut state);
        let initial = state.gain_snapshot();
        assert!(initial.can_set_gain());
        assert_eq!(initial.last_sent, None);
        assert_eq!(state.gain.pending, None);

        assert!(state.submit_gain(0, GainLevel::Mid));
        assert_eq!(state.gain.pending.take(), Some(GainLevel::Mid));
        let sending = state.gain_snapshot();
        assert_eq!(sending.requested, Some(GainLevel::Mid));
        assert_eq!(sending.last_sent, None);
        assert!(!sending.can_set_gain());
        assert!(!state.submit_gain(0, GainLevel::Low));
        assert!(state.complete(
            0,
            Work::WriteGain(GainLevel::Mid),
            Ok(Completed::Gain(GainLevel::Mid)),
        ));
        let sent = state.gain_snapshot();
        assert!(sent.can_set_gain());
        assert_eq!(sent.requested, None);
        assert_eq!(sent.last_sent, Some(GainLevel::Mid));

        // Volume changes and polling on the same handle leave gain history intact.
        state.confirm(VolumeReading {
            reg_10: 0x5c,
            reg_11: 0x5d,
        });
        assert!(!state.snapshot.adjustable());
        assert!(state.gain_snapshot().can_set_gain());
        assert_eq!(state.gain_snapshot().last_sent, Some(GainLevel::Mid));
        ready(&mut state);
        assert_eq!(state.gain_snapshot().last_sent, Some(GainLevel::Mid));
    }

    #[test]
    fn partial_gain_failure_clears_request_and_retains_error_after_recovery() {
        let mut state = State::new();
        ready(&mut state);
        assert!(state.submit_gain(0, GainLevel::Low));
        assert_eq!(state.gain.pending.take(), Some(GainLevel::Low));
        state.complete(
            0,
            Work::WriteGain(GainLevel::Low),
            Ok(Completed::Gain(GainLevel::Low)),
        );
        assert!(state.submit_gain(0, GainLevel::High));
        assert_eq!(state.gain.pending.take(), Some(GainLevel::High));
        assert!(state.complete(
            0,
            Work::WriteGain(GainLevel::High),
            Err(Error::GainWriteUnknown {
                command: 0x16,
                address: 0x62,
                previous_reply: Some((0x15, 0x60)),
                source: Box::new(Error::Timeout),
            }),
        ));
        let failed = state.gain_snapshot();
        assert_eq!(failed.status, Status::Sync);
        assert!(!failed.can_set_gain());
        assert_eq!(failed.requested, None);
        assert_eq!(failed.last_sent, None);
        let detail = failed.error.unwrap();
        assert!(detail.contains("Gain outcome unknown"));
        assert!(detail.contains("First report (command 0x15, DAC 0x60) received a matching reply"));
        assert_eq!(state.gain.pending, None);

        assert!(state.complete(
            failed.epoch,
            Work::Open,
            Ok(Completed::Volume(VolumeReading {
                reg_10: 0x5c,
                reg_11: 0x5c
            })),
        ));
        let recovered = state.gain_snapshot();
        assert!(recovered.can_set_gain());
        assert_eq!(recovered.error.as_deref(), Some(detail.as_str()));
        assert!(state.last_error.is_none());
        assert!(state.submit_gain(recovered.epoch, GainLevel::Mid));
        assert!(state.gain_snapshot().error.is_none());
    }

    #[test]
    fn reconnect_drops_queued_gain_and_rejects_a_late_result_during_new_write() {
        let mut state = State::new();
        ready(&mut state);
        assert!(state.submit_gain(0, GainLevel::High));
        state.invalidate(Status::Reading);
        assert_eq!(state.gain.pending, None);
        assert!(!state.gain_snapshot().can_set_gain());

        ready(&mut state);
        assert!(state.submit_gain(1, GainLevel::Low));
        assert_eq!(state.gain.pending.take(), Some(GainLevel::Low));
        state.complete(
            1,
            Work::WriteGain(GainLevel::Low),
            Ok(Completed::Gain(GainLevel::Low)),
        );
        assert!(state.submit_gain(1, GainLevel::High));
        assert_eq!(state.gain.pending.take(), Some(GainLevel::High));
        state.invalidate(Status::Reading);
        assert!(!state.active(1));
        assert_eq!(state.gain_snapshot().last_sent, None);
        assert_eq!(state.gain_snapshot().requested, None);
        ready(&mut state);
        assert!(!state.submit_gain(1, GainLevel::High));
        assert!(state.submit_gain(2, GainLevel::Mid));
        for late in [Ok(Completed::Gain(GainLevel::High)), Err(Error::Timeout)] {
            assert!(!state.complete(1, Work::WriteGain(GainLevel::High), late));
        }
        let current = state.gain_snapshot();
        assert_eq!(current.requested, Some(GainLevel::Mid));
        assert_eq!(current.last_sent, None);
        assert!(current.error.is_none());
        assert!(!current.can_set_gain());
        assert_eq!(state.gain.pending, Some(GainLevel::Mid));
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
