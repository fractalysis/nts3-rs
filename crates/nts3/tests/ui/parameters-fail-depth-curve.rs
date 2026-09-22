use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadDepthCurve {
    #[parameter(name="MIX", min=-1000, max=1000, default=0, parameter_type="drywet", assign="depth")]
    value: Parameter,
}
fn main() {}
