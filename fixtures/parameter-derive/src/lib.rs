#![no_std]

use nts3::prelude::*;

/// Target-check fixture matching Smooth Echo's normative parameter declaration.
#[derive(Nts3Parameters)]
pub struct EchoParameters {
    #[parameter(
        name = "TIME",
        min = 1,
        max = 2000,
        default = 500,
        parameter_type = "milliseconds",
        smoothing_ms = 100.0,
        assign = "x",
        curve = "exp",
        mapping_min = 1,
        mapping_max = 2000,
        mapping_default = 500
    )]
    pub time_ms: SmoothedParameter,

    #[parameter(
        name = "FEEDBACK",
        min = 0,
        max = 1000,
        default = 0,
        parameter_type = "percent",
        decimal_places = 1,
        assign = "y",
        curve = "linear",
        mapping_min = 0,
        mapping_max = 1000,
        mapping_default = 0
    )]
    pub feedback: Parameter,

    #[parameter(
        name = "DEPTH",
        min = -1000,
        max = 1000,
        center = 0,
        default = 1000,
        parameter_type = "drywet",
        decimal_places = 1,
        assign = "depth",
        curve = "exp",
        curve_polarity = "bipolar",
        mapping_min = -1000,
        mapping_max = 1000,
        mapping_default = 1000
    )]
    pub depth: Parameter,
}
