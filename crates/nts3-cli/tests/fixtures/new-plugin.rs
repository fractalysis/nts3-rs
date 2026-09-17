#![no_std]

use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {
    #[parameter(name = "GAIN", min = 0, max = 100, default = 100, parameter_type = "percent")]
    gain: Parameter,
}

#[derive(Default)]
struct Plugin;

#[nts3::plugin(
    name = "Test Plug",
    developer_id = 0x5255_5354,
    unit_id = 0x12345678,
    sdram_bytes = 1
)]
impl Nts3Plugin for Plugin {
    type Parameters = Parameters;

    fn process(&mut self, parameters: &mut Parameters, buffer: &mut StereoBuffer<'_>) {
        let gain = parameters.gain.normalized();
        for mut frame in buffer.frames_mut() {
            let [left, right] = frame.input();
            frame.write([left * gain, right * gain]);
        }
    }
}
