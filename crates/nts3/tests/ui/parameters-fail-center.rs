use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadCenter {
    #[parameter(name="BAD", min=-10, max=10, center=11, default=0, parameter_type="none")]
    value: Parameter,
}
fn main() {}
