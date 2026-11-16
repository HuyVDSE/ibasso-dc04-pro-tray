use hidapi::{DeviceInfo, HidApi, HidDevice};
use std::time::{Duration, Instant};

use super::{Error, protocol};

const VID: u16 = 0x262a;
const PID: u16 = 0x187f;
const USAGE_PAGE: u16 = 0x000c;
const USAGE: u16 = 0x0001;
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(500);

pub fn collections(api: &HidApi) -> impl Iterator<Item = &DeviceInfo> {
    api.device_list()
        .filter(|info| info.vendor_id() == VID && info.product_id() == PID)
}

pub(super) fn is_control_collection(info: &DeviceInfo) -> bool {
    info.usage_page() == USAGE_PAGE && info.usage() == USAGE
}

pub(super) fn query(handle: &HidDevice, output: &[u8], command: u8) -> Result<u8, Error> {
    exchange(handle, output, "sending a query output report", |input| {
        protocol::response_byte(input, command)
    })
}

pub(super) fn write_volume(handle: &HidDevice, output: &[u8; 17]) -> Result<(), Error> {
    exchange(handle, output, "sending a volume output report", |input| {
        protocol::matching_response(input, output[1]).map(|_| ())
    })
}

fn exchange<T>(
    handle: &HidDevice,
    output: &[u8],
    write_operation: &'static str,
    parse: impl Fn(&[u8]) -> Option<T>,
) -> Result<T, Error> {
    let written = handle.write(output).map_err(|source| Error::Hid {
        operation: write_operation,
        source,
    })?;
    if written < output.len() {
        return Err(Error::ShortWrite {
            expected: output.len(),
            written,
        });
    }

    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    let mut input = [0; 256];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(Error::Timeout);
        }
        let size = handle
            .read_timeout(&mut input, remaining.as_millis().max(1) as i32)
            .map_err(|source| Error::Hid {
                operation: "reading the input report",
                source,
            })?;
        if size == 0 {
            return Err(Error::Timeout);
        }
        if let Some(value) = parse(&input[..size]) {
            return Ok(value);
        }
    }
}
