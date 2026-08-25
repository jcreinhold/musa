//! The audited exact-quantity → floating DSP boundary.
//!
//! Written values stay rational through parsing, resolution, facts, and graph
//! intent. Plan preparation calls only these functions. Division uses IEEE-754
//! round-to-nearest; transcendental decibel conversion then uses the platform
//! `powf`, as DSP coefficients necessarily do.

use num_rational::Ratio;

use crate::{Unit, WrittenQuantity};

pub(crate) fn ratio_to_f64(value: Ratio<i64>) -> f64 {
    (*value.numer() as f64) / (*value.denom() as f64)
}

pub(crate) fn quantity_to_f64(value: WrittenQuantity) -> f64 {
    ratio_to_f64(value.magnitude)
}

pub(crate) fn linear(value: WrittenQuantity) -> f64 {
    let magnitude = quantity_to_f64(value);
    match value.unit {
        Unit::Decibels => 10f64.powf(magnitude / 20.0),
        Unit::Hz | Unit::Linear | Unit::Seconds => magnitude,
    }
}

pub(crate) fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

pub(crate) fn linear_f32(value: WrittenQuantity) -> f32 {
    f64_to_f32(linear(value))
}
