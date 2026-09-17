use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct TooWide {
    #[parameter(name="BAD", min=0, max=40000, default=0, parameter_type="none")]
    value: Parameter,
}
fn main() {}
