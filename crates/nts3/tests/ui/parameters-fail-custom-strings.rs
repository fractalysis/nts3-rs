use nts3::prelude::*;
#[derive(Nts3Parameters)]
struct DeferredStrings {
    #[parameter(name="MODE", min=0, max=1, default=0, parameter_type="strings")]
    value: Parameter,
}
fn main() {}
