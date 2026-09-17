use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct MissingSmoothing {
    #[parameter(name="BAD", min=0, max=1, default=0, parameter_type="none")]
    value: SmoothedParameter,
}
fn main() {}
