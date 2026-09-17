use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadRange {
    #[parameter(name="BAD", min=10, max=1, default=5, parameter_type="none")]
    value: Parameter,
}
fn main() {}
