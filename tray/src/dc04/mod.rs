mod hid;
mod protocol;

use hidapi::{HidApi, HidDevice, HidError};
use std::{error::Error as StdError, fmt};

pub use hid::collections;

pub struct Device {
    handle: HidDevice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeReading {
    pub reg_10: u8,
    pub reg_11: u8,
}

impl VolumeReading {
    pub fn step(self) -> u8 {
        // Balance reduces the code on one channel and leaves the base code on the other.
        protocol::volume_step(self.reg_10.max(self.reg_11))
    }

    pub fn centered(self) -> bool {
        self.reg_10 == self.reg_11 && self.reg_10 != 0
    }

    pub fn adjustable(self) -> bool {
        self.centered() && (MIN_TARGET..=MAX_TARGET).contains(&self.step())
    }
}

// Until the edge transitions have been verified on hardware.
pub const MIN_TARGET: u8 = 1;
pub const MAX_TARGET: u8 = 99;

impl fmt::Display for VolumeReading {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "step={} reg_0x10=0x{:02X} reg_0x11=0x{:02X}",
            self.step(),
            self.reg_10,
            self.reg_11
        )
    }
}

pub enum VolumeDownEvent {
    Planned {
        before: VolumeReading,
        target: VolumeReading,
    },
    LeftStoreReply,
}

#[derive(Clone, Copy)]
enum Target {
    Down,
    Step(u8),
    BenchEdge(u8),
}

impl Device {
    pub fn open(api: &HidApi) -> Result<Self, Error> {
        let mut matches = collections(api).filter(|info| hid::is_control_collection(info));
        let info = matches.next().ok_or(Error::NoDevice)?;

        if matches.next().is_some() {
            return Err(Error::MultipleDevices);
        }

        Ok(Self {
            handle: info.open_device(api).map_err(|source| Error::Hid {
                operation: "opening the DC04 Pro HID collection",
                source,
            })?,
        })
    }

    /// Read the FPGA version over the same HID collection used by the iBasso web app.
    pub fn fpga_version(&self) -> Result<u8, Error> {
        hid::query(&self.handle, &protocol::FPGA_QUERY, protocol::FPGA_COMMAND)
    }

    pub fn volume(&self) -> Result<u8, Error> {
        Ok(self.volume_reading()?.step())
    }

    pub fn volume_reading(&self) -> Result<VolumeReading, Error> {
        let [first, second] = protocol::VOLUME_REGISTERS;
        Ok(VolumeReading {
            reg_10: self.volume_register(first)?,
            reg_11: self.volume_register(second)?,
        })
    }

    /// Lower the volume by one step at centered balance and verify both registers.
    /// Consumes the handle so another attempt opens a new session and reads again.
    pub fn volume_down(
        self,
        progress: impl FnMut(VolumeDownEvent),
    ) -> Result<VolumeReading, Error> {
        change_volume(
            || self.volume_reading(),
            |report| hid::write_volume(&self.handle, report),
            Target::Down,
            progress,
            || true,
        )
    }

    /// Check both registers before writing, then verify both codes after writing.
    /// The session owner can cancel between output reports.
    pub fn set_volume(
        &self,
        target: u8,
        active: impl Fn() -> bool,
    ) -> Result<VolumeReading, Error> {
        self.change(Target::Step(target), active)
    }

    /// Explicit CLI bench operation: 0 and 100 are not exposed by the tray.
    /// Only leave 0 after a fresh FF/FF reading. Never write from 00/00.
    pub fn bench_edge(&self, target: u8) -> Result<VolumeReading, Error> {
        self.change(Target::BenchEdge(target), || true)
    }

    fn change(&self, target: Target, active: impl Fn() -> bool) -> Result<VolumeReading, Error> {
        change_volume(
            || self.volume_reading(),
            |report| hid::write_volume(&self.handle, report),
            target,
            |_| {},
            active,
        )
    }

    fn volume_register(&self, register: u8) -> Result<u8, Error> {
        hid::query(
            &self.handle,
            &protocol::volume_query(register),
            protocol::VOLUME_COMMAND,
        )
    }
}

fn change_volume(
    mut read: impl FnMut() -> Result<VolumeReading, Error>,
    mut write: impl FnMut(&[u8; 17]) -> Result<(), Error>,
    requested: Target,
    mut progress: impl FnMut(VolumeDownEvent),
    active: impl Fn() -> bool,
) -> Result<VolumeReading, Error> {
    if !active() {
        return Err(Error::SessionEnded);
    }
    let before = read().map_err(|source| Error::ReadBeforeWrite(Box::new(source)))?;
    let target_step = match requested {
        Target::Down => before.step().checked_sub(1).ok_or(Error::AtMinimum)?,
        Target::Step(step) if (MIN_TARGET..=MAX_TARGET).contains(&step) => step,
        Target::BenchEdge(step @ (0 | 100)) if before.adjustable() => step,
        Target::BenchEdge(1) if before.step() == 0 => 1,
        _ => return Err(Error::UnverifiedEdge),
    };
    // Equal, nonzero codes cannot be the result of clipping a nonzero balance
    // offset to zero. Unequal codes could also be left by a partial write.
    if !before.centered() {
        return Err(Error::UnconfirmedCenter(before));
    }
    if matches!(requested, Target::Step(_)) && !before.adjustable() {
        return Err(Error::UnverifiedEdge);
    }
    let code = protocol::volume_code(target_step);
    let target = VolumeReading {
        reg_10: code,
        reg_11: code,
    };
    progress(VolumeDownEvent::Planned { before, target });
    if matches!(requested, Target::Step(_)) && before == target {
        return Ok(before);
    }
    for report in &protocol::volume_writes([code, code]) {
        if !active() {
            return Err(Error::SessionEnded);
        }
        let command = report[1];
        write(report).map_err(|source| Error::VolumeWriteUnknown {
            command,
            source: Box::new(source),
        })?;
        if command == protocol::LEFT_VOLUME_STORE {
            progress(VolumeDownEvent::LeftStoreReply);
        }
    }

    if !active() {
        return Err(Error::SessionEnded);
    }
    let observed = read().map_err(|source| Error::ReadBackUnknown(Box::new(source)))?;
    if observed != target {
        return Err(Error::UnexpectedReadBack {
            expected: target,
            observed,
        });
    }
    Ok(observed)
}

const OUTCOME_UNKNOWN: &str =
    "Volume outcome unknown. Reconnect if needed and run dc04-cli get --raw before another write";

#[derive(Debug)]
pub enum Error {
    NoDevice,
    MultipleDevices,
    Hid {
        operation: &'static str,
        source: HidError,
    },
    Timeout,
    ShortWrite {
        expected: usize,
        written: usize,
    },
    AtMinimum,
    UnverifiedEdge,
    SessionEnded,
    UnconfirmedCenter(VolumeReading),
    ReadBeforeWrite(Box<Error>),
    VolumeWriteUnknown {
        command: u8,
        source: Box<Error>,
    },
    ReadBackUnknown(Box<Error>),
    UnexpectedReadBack {
        expected: VolumeReading,
        observed: VolumeReading,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => {
                formatter.write_str("No DC04 Pro control collection (262a:187f, 000c:0001) found")
            }
            Self::MultipleDevices => {
                formatter.write_str("Multiple DC04 Pro control collections found. Connect one DAC.")
            }
            Self::Hid { operation, source } => {
                write!(formatter, "HID error while {operation}: {source}")
            }
            Self::Timeout => {
                formatter.write_str("Timed out waiting for the matching DC04 Pro input report")
            }
            Self::ShortWrite { expected, written } => write!(
                formatter,
                "Incomplete DC04 Pro output report: wrote {written} of {expected} bytes"
            ),
            Self::AtMinimum => formatter.write_str("Already at step 0; no volume write sent."),
            Self::UnverifiedEdge => formatter.write_str(
                "Transition involving step 0 or 100 is not enabled; no volume write sent.",
            ),
            Self::SessionEnded => formatter.write_str("The active DAC session ended."),
            Self::UnconfirmedCenter(reading) => write!(
                formatter,
                "Cannot confirm centered balance ({reading}); no volume write sent. Center the balance and read again at a low, nonzero step."
            ),
            Self::ReadBeforeWrite(source) => write!(
                formatter,
                "Current DAC state unknown; no volume write attempted: {source}"
            ),
            Self::VolumeWriteUnknown { command, source } => write!(
                formatter,
                "{OUTCOME_UNKNOWN}. Volume command 0x{command:02X}: {source}"
            ),
            Self::ReadBackUnknown(source) => {
                write!(
                    formatter,
                    "{OUTCOME_UNKNOWN}. Cannot read back both registers: {source}"
                )
            }
            Self::UnexpectedReadBack { expected, observed } => write!(
                formatter,
                "{OUTCOME_UNKNOWN}. Expected {expected}; read back {observed}."
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Hid { source, .. } => Some(source),
            Self::ReadBeforeWrite(source)
            | Self::VolumeWriteUnknown { source, .. }
            | Self::ReadBackUnknown(source) => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE: VolumeReading = VolumeReading {
        reg_10: 0x5c,
        reg_11: 0x5c,
    };
    const TARGET: VolumeReading = VolumeReading {
        reg_10: 0x5d,
        reg_11: 0x5d,
    };

    fn lower_one(
        read: impl FnMut() -> Result<VolumeReading, Error>,
        write: impl FnMut(&[u8; 17]) -> Result<(), Error>,
        progress: impl FnMut(VolumeDownEvent),
    ) -> Result<VolumeReading, Error> {
        change_volume(read, write, Target::Down, progress, || true)
    }

    #[test]
    fn lowers_one_step_with_vendor_reports_and_fresh_readback() {
        let mut reads = [BEFORE, TARGET].into_iter();
        let mut sent = Vec::new();
        let result = lower_one(
            || Ok(reads.next().unwrap()),
            |report| {
                sent.push(*report);
                Ok(())
            },
            |_| {},
        )
        .unwrap();

        assert_eq!((BEFORE.step(), result.step()), (28, 27));
        assert!(reads.next().is_none());
        assert_eq!(
            sent.iter().map(|report| report[1]).collect::<Vec<_>>(),
            [0x01, 0x02, 0x09, 0x0a, 0x13, 0x03, 0x04, 0x0b, 0x0c, 0x14]
        );
        assert_eq!(
            sent[0],
            [
                0, 0x01, 0x11, 0x88, 0x60, 0, 0, 5, 9, 0, 1, 0, 0x5d, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            sent[4],
            [
                0, 0x13, 0x11, 0xa0, 0xa2, 0, 0x10, 1, 0x5d, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            sent[9],
            [
                0, 0x14, 0x11, 0xa0, 0xa2, 0, 0x11, 1, 0x5d, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn sends_no_volume_report_at_minimum_or_with_uncertain_balance() {
        let minimum = VolumeReading {
            reg_10: 0xff,
            reg_11: 0xff,
        };
        let result = lower_one(|| Ok(minimum), |_| panic!("unexpected write"), |_| {});
        assert!(matches!(result, Err(Error::AtMinimum)));
        for reading in [
            VolumeReading {
                reg_11: 0x5b,
                ..BEFORE
            },
            VolumeReading {
                reg_10: 0,
                reg_11: 0,
            },
        ] {
            let result = lower_one(|| Ok(reading), |_| panic!("unexpected write"), |_| {});
            assert!(matches!(result, Err(Error::UnconfirmedCenter(_))));
        }
    }

    #[test]
    fn timeout_after_left_store_stops_without_retry_or_readback() {
        let mut reads = [BEFORE].into_iter();
        let mut sent = Vec::new();
        let result = lower_one(
            || Ok(reads.next().unwrap()),
            |report| {
                sent.push(report[1]);
                if report[1] == 0x03 {
                    Err(Error::Timeout)
                } else {
                    Ok(())
                }
            },
            |_| {},
        );
        assert!(matches!(
            result,
            Err(Error::VolumeWriteUnknown { command: 0x03, .. })
        ));
        assert_eq!(sent, [0x01, 0x02, 0x09, 0x0a, 0x13, 0x03]);
    }

    #[test]
    fn refuses_to_confirm_when_only_one_register_matches() {
        let partial = VolumeReading {
            reg_11: BEFORE.reg_11,
            ..TARGET
        };
        let mut reads = [BEFORE, partial].into_iter();
        let result = lower_one(|| Ok(reads.next().unwrap()), |_| Ok(()), |_| {});
        assert!(
            matches!(result, Err(Error::UnexpectedReadBack { expected: TARGET, observed }) if observed == partial)
        );
    }

    #[test]
    fn arbitrary_increase_writes_and_verifies_the_requested_code() {
        let target = VolumeReading {
            reg_10: 0x54,
            reg_11: 0x54,
        };
        let mut reads = [BEFORE, target].into_iter();
        let mut stored = Vec::new();
        let verified = change_volume(
            || Ok(reads.next().unwrap()),
            |report| {
                if matches!(report[1], 0x13 | 0x14) {
                    stored.push((report[6], report[8]));
                }
                Ok(())
            },
            Target::Step(33),
            |_| {},
            || true,
        )
        .unwrap();
        assert_eq!(verified, target);
        assert_eq!(stored, [(0x10, 0x54), (0x11, 0x54)]);
    }

    #[test]
    fn endpoints_cannot_be_selected_normally_and_zero_code_cannot_enable_writes() {
        let minimum = VolumeReading {
            reg_10: 255,
            reg_11: 255,
        };
        let maximum = VolumeReading {
            reg_10: 0,
            reg_11: 0,
        };
        for (reading, target) in [
            (BEFORE, Target::Step(0)),
            (BEFORE, Target::Step(100)),
            (minimum, Target::Step(1)),
            (maximum, Target::Step(99)),
            (maximum, Target::BenchEdge(1)),
        ] {
            assert!(
                change_volume(
                    || Ok(reading),
                    |_| panic!("unexpected write"),
                    target,
                    |_| {},
                    || true,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn session_invalidation_between_reports_stops_at_the_last_in_flight_report() {
        let active = std::cell::Cell::new(true);
        let mut reads = [BEFORE].into_iter();
        let mut sent = Vec::new();
        let result = change_volume(
            || Ok(reads.next().unwrap()),
            |report| {
                sent.push(report[1]);
                if report[1] == protocol::LEFT_VOLUME_STORE {
                    active.set(false);
                }
                Ok(())
            },
            Target::Step(27),
            |_| {},
            || active.get(),
        );
        assert!(matches!(result, Err(Error::SessionEnded)));
        assert_eq!(sent, [0x01, 0x02, 0x09, 0x0a, 0x13]);
    }
}
