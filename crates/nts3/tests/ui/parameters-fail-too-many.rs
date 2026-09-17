use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct TooMany {
    #[parameter(name="A", min=0, max=1, default=0, parameter_type="none")] a: Parameter,
    #[parameter(name="B", min=0, max=1, default=0, parameter_type="none")] b: Parameter,
    #[parameter(name="C", min=0, max=1, default=0, parameter_type="none")] c: Parameter,
    #[parameter(name="D", min=0, max=1, default=0, parameter_type="none")] d: Parameter,
    #[parameter(name="E", min=0, max=1, default=0, parameter_type="none")] e: Parameter,
    #[parameter(name="F", min=0, max=1, default=0, parameter_type="none")] f: Parameter,
    #[parameter(name="G", min=0, max=1, default=0, parameter_type="none")] g: Parameter,
    #[parameter(name="H", min=0, max=1, default=0, parameter_type="none")] h: Parameter,
    #[parameter(name="I", min=0, max=1, default=0, parameter_type="none")] i: Parameter,
}
fn main() {}
