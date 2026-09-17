use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {}
#[derive(Default)]
struct Plugin;

#[nts3::plugin(name = "Reserved", developer_id = 0, unit_id = 1, sdram_bytes = 1024)]
impl Nts3Plugin for Plugin {
    type Parameters = Parameters;
    fn process(&mut self, _: &mut Self::Parameters, _: &mut StereoBuffer<'_>) {}
}

fn main() {}
