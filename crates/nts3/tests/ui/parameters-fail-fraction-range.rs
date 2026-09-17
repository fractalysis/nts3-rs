use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct BadFractionRange {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none", decimal_places=16)]
    value: Parameter,
}
fn main() {}
