//! Asset exposure metric calculator.
//!
//! Computes effective exposure by `AssetType` based on constituent weights.

use crate::instruments::exotics::basket::types::{AssetType, Basket, ConstituentReference};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_quant_core::Result;

/// Calculate effective exposure by asset type
pub struct AssetExposureCalculator {
    pub(crate) asset_type: AssetType,
}

impl AssetExposureCalculator {
    /// Create an asset exposure calculator for the given asset type
    pub fn new(asset_type: AssetType) -> Self {
        Self { asset_type }
    }
}

impl MetricCalculator for AssetExposureCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let basket = context.instrument_as::<Basket>()?;
        let mut total_exposure = 0.0;
        for (index, constituent) in basket.constituents.iter().enumerate() {
            let matches = match (&constituent.reference, &self.asset_type) {
                (ConstituentReference::MarketData { asset_type, .. }, target) => {
                    std::mem::discriminant(asset_type) == std::mem::discriminant(target)
                }
                (ConstituentReference::Instrument(_), target) => {
                    use crate::pricer::InstrumentType;
                    let boxed = basket.boxed_constituent_at(index)?.ok_or(
                        finstack_quant_core::Error::Input(
                            finstack_quant_core::InputError::NotFound {
                                id: constituent.id.clone(),
                            },
                        ),
                    )?;
                    let it = boxed.key();
                    matches!(
                        (it, target),
                        (InstrumentType::Bond, AssetType::Bond)
                            | (InstrumentType::Equity, AssetType::Equity)
                            | (InstrumentType::Basket, AssetType::ETF)
                    )
                }
            };
            if matches {
                total_exposure += constituent.weight;
            }
        }
        Ok(total_exposure * 100.0)
    }
}
