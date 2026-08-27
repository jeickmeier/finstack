//! WASM bindings for dynamic term-structure models.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Evaluate the static Nelson-Siegel (1987) yield curve for one factor triple.
///
/// This is the Diebold-Li cross-sectional equation for a single date:
/// `y(tau) = b1 + b2 * s(tau) + b3 * (s(tau) - exp(-lambda * tau))` with
/// `s(tau) = (1 - exp(-lambda * tau)) / (lambda * tau)`. Returns one yield per
/// tenor, in decimal units and in input order.
/// @param lambda - Exponential decay parameter for tenors in years; must be finite and greater than zero (0.7308 is the years-equivalent of Diebold-Li's 0.0609 months value).
/// @param level - Nelson-Siegel beta1, the long-run level factor in decimal yield units such as 0.06 for 6%.
/// @param slope - Nelson-Siegel beta2, the slope factor (negative of the short-minus-long spread) in decimal yield units.
/// @param curvature - Nelson-Siegel beta3, the hump-shaped curvature factor in decimal yield units.
/// @param tenors - Maturities in years, each finite and non-negative; output order matches this array.
/// @returns One decimal yield per tenor, in the same order as `tenors`.
///
/// # Errors
///
/// Throws a JavaScript exception if `lambda` is non-finite or non-positive, any
/// factor loading is non-finite, or any tenor is non-finite or negative.
#[wasm_bindgen(js_name = nelsonSiegelYields)]
pub fn nelson_siegel_yields(
    lambda: f64,
    level: f64,
    slope: f64,
    curvature: f64,
    tenors: &[f64],
) -> Result<Box<[f64]>, JsValue> {
    finstack_quant_models::rates::dtsm::nelson_siegel_yields(
        lambda,
        [level, slope, curvature],
        tenors,
    )
    .map(Vec::into_boxed_slice)
    .map_err(to_js_err)
}

#[cfg(test)]
mod tests {
    use super::nelson_siegel_yields;

    #[test]
    fn nelson_siegel_yields_preserves_model_values() {
        let actual = nelson_siegel_yields(0.7308, 0.03, -0.01, 0.005, &[1.0, 5.0, 10.0])
            .expect("valid Nelson-Siegel inputs");
        let expected = [
            0.024_045_061_287_046_227,
            0.028_537_623_036_310_49,
            0.029_312_926_009_712_76,
        ];
        for (value, expected_value) in actual.iter().zip(expected) {
            assert!((value - expected_value).abs() < 1e-15);
        }
    }
}
