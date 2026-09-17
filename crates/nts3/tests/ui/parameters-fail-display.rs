use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadDisplay {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="bananas")]
    value: Parameter,
}
fn main() {}
