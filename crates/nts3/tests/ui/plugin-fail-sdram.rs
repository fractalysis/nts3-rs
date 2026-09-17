use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {}
#[derive(Default)]
struct Plugin;

#[nts3::plugin(
    name = "Too Much Memory",
    developer_id = 0x5255_5354,
    unit_id = 1,
    sdram_bytes = 3_145_729
)]
impl Nts3Plugin for Plugin {
    type Parameters = Parameters;
    fn process(&mut self, _: &mut Self::Parameters, _: &mut StereoBuffer<'_>) {}
}

fn main() {}
