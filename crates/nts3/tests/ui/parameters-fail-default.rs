use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadDefault {
    #[parameter(name="BAD", min=0, max=10, default=11, parameter_type="none")]
    value: Parameter,
}
fn main() {}
