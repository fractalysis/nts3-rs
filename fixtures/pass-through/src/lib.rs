#![no_std]

use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct PassThroughParameters {
    #[parameter(name = "TEST", min = 0, max = 1, default = 0, parameter_type = "none")]
    test: Parameter,
}

#[derive(Default)]
struct PassThrough;

#[nts3::plugin(
    name = "Rust Pass Through",
    developer_id = 0x5255_5354,
    unit_id = 0x5041_5353,
    sdram_bytes = 1
)]
impl Nts3Plugin for PassThrough {
    type Parameters = PassThroughParameters;

    fn process(&mut self, _parameters: &mut Self::Parameters, buffer: &mut StereoBuffer<'_>) {
        for mut frame in buffer.frames_mut() {
            frame.write(frame.input());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_header_matches_fixture_metadata() {
        let common = unit_header.common();
        assert_eq!(common.name(), *b"Rust Pass Through\0\0\0");
        assert_eq!(common.developer_id(), 0x5255_5354);
        assert_eq!(common.unit_id(), 0x5041_5353);
        assert_eq!(common.version(), 0x0000_0100);
        assert_eq!(common.num_params(), 1);
        assert_eq!(nts3_resources.sdram_bytes(), 1);
    }
}
