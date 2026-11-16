pub(super) const FPGA_COMMAND: u8 = 0x58;
pub(super) const VOLUME_COMMAND: u8 = 0x41;
pub(super) const VOLUME_REGISTERS: [u8; 2] = [0x10, 0x11];
pub(super) const LEFT_VOLUME_STORE: u8 = 0x13;

// Index is the iBasso slider position. Value is the DAC's attenuation code.
const VOLUME_TABLE: [u8; 101] = [
    255, 155, 150, 145, 140, 135, 130, 125, 120, 115, 110, 109, 108, 107, 106, 105, 104, 103, 102,
    101, 100, 99, 98, 97, 96, 95, 94, 93, 92, 91, 90, 88, 86, 84, 82, 80, 78, 76, 74, 72, 70, 68,
    66, 64, 62, 60, 58, 56, 54, 52, 50, 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39, 38, 37, 36, 35,
    34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11,
    10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
];

// HIDAPI takes the report ID (0 for an unnumbered report) followed by the payload.
pub(super) const FPGA_QUERY: [u8; 17] = [
    0,
    FPGA_COMMAND,
    0x12,
    0xdc,
    0x2c,
    4,
    0,
    1,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
];

pub(super) fn volume_query(register: u8) -> [u8; 17] {
    [
        0,
        VOLUME_COMMAND,
        0x12,
        0xe4,
        0xa2,
        0,
        register,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

pub(super) fn response_byte(input: &[u8], command: u8) -> Option<u8> {
    let payload = matching_response(input, command)?;
    (payload[7] == 1).then_some(payload[8])
}

pub(super) fn matching_response(input: &[u8], command: u8) -> Option<&[u8; 32]> {
    // The Windows HIDAPI backend strips the leading zero from unnumbered input.
    let payload: &[u8; 32] = input.try_into().ok()?;
    let is_button_event = payload[4..6] == [0xfe, 0x01];
    (!is_button_event && payload[6] == command).then_some(payload)
}

pub(super) fn volume_step(attenuation: u8) -> u8 {
    // Match the Android controller's rounding when the code falls between two entries.
    VOLUME_TABLE.partition_point(|&code| code > attenuation) as u8
}

pub(super) fn volume_code(step: u8) -> u8 {
    VOLUME_TABLE[usize::from(step)]
}

// Use the DC04 Pro WebHID controller's order: four writes to each DAC
// followed by its corresponding 0x10/0x11 register write.
pub(super) fn volume_writes([left, right]: [u8; 2]) -> [[u8; 17]; 10] {
    [
        dac_write(0x01, 0x60, 0x09, 1, left),
        dac_write(0x02, 0x60, 0x09, 2, left),
        dac_write(0x09, 0x60, 0x07, 0, left),
        dac_write(0x0a, 0x60, 0x07, 1, left),
        store_write(LEFT_VOLUME_STORE, VOLUME_REGISTERS[0], left),
        dac_write(0x03, 0x62, 0x09, 1, right),
        dac_write(0x04, 0x62, 0x09, 2, right),
        dac_write(0x0b, 0x62, 0x07, 0, right),
        dac_write(0x0c, 0x62, 0x07, 1, right),
        store_write(0x14, VOLUME_REGISTERS[1], right),
    ]
}

fn dac_write(command: u8, address: u8, register: u8, channel: u8, code: u8) -> [u8; 17] {
    [
        0, command, 0x11, 0x88, address, 0, 0, 5, register, 0, channel, 0, code, 0, 0, 0, 0,
    ]
}

fn store_write(command: u8, register: u8, code: u8) -> [u8; 17] {
    [
        0, command, 0x11, 0xa0, 0xa2, 0, register, 1, code, 0, 0, 0, 0, 0, 0, 0, 0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_captured_volume_reply() {
        let mut input = [0; 32];
        input[6..10].copy_from_slice(&[0x41, 0x01, 0x5c, 0x5c]);

        assert_eq!(
            response_byte(&input, VOLUME_COMMAND).map(volume_step),
            Some(28)
        );
        assert_eq!(response_byte(&input, FPGA_COMMAND), None);
        assert_eq!(response_byte(&input[..9], VOLUME_COMMAND), None);

        input[7] = 2;
        assert_eq!(response_byte(&input, VOLUME_COMMAND), None);
        input[7] = 1;
        input[4..6].copy_from_slice(&[0xfe, 0x01]);
        assert_eq!(response_byte(&input, VOLUME_COMMAND), None);

        input[4..6].fill(0);
        input[8..10].copy_from_slice(&[0x4c, 0]);
        assert_eq!(
            response_byte(&input, VOLUME_COMMAND).map(volume_step),
            Some(37)
        );
    }

    #[test]
    fn maps_the_display_range_and_rounds_between_entries() {
        assert_eq!(volume_step(255), 0);
        assert_eq!(volume_step(154), 2);
        assert_eq!(volume_step(50), 50);
        assert_eq!(volume_step(0), 100);
    }

    #[test]
    fn accepts_write_reply_without_assuming_a_data_byte() {
        let mut input = [0; 32];
        input[6] = LEFT_VOLUME_STORE;
        assert!(matching_response(&input, LEFT_VOLUME_STORE).is_some());
        assert!(matching_response(&input, 0x14).is_none());
        assert!(response_byte(&input, LEFT_VOLUME_STORE).is_none());

        input[4..6].copy_from_slice(&[0xfe, 0x01]);
        assert!(matching_response(&input, LEFT_VOLUME_STORE).is_none());
    }
}
