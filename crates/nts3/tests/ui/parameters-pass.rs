use nts3::prelude::*;

#[derive(Nts3Parameters)]
struct Parameters {
    #[parameter(name = "LEVEL", min = -100, max = 100, center = 0, default = 0,
        parameter_type = "percent", decimal_places = 1, assign = "depth",
        curve = "exp", curve_polarity = "bipolar", mapping_min = -100,
        mapping_max = 100, mapping_default = 0)]
    level: Parameter,
    #[parameter(name = "RATE", min = 1, max = 1000, default = 10,
        parameter_type = "milliseconds", smoothing_ms = 5.0, assign = "x",
        curve = "log")]
    rate: SmoothedParameter,
}

fn main() {
    let parameters = Parameters::default();
    assert_eq!(Parameters::COUNT, 2);
    assert_eq!(parameters.level.raw(), 0);
    assert_eq!(parameters.rate.raw(), 10);
}
