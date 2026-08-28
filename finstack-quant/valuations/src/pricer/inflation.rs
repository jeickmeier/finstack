//! Pricer registrations for inflation instruments.
//!
//! Covers: InflationSwap, YoYInflationSwap, InflationCapFloor.

use super::{register_generic, InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for inflation instruments (swaps, caps/floors).
pub(crate) fn register_inflation_pricers(
    registry: &mut PricerRegistry,
) -> std::result::Result<(), crate::pricer::PricingError> {
    // Inflation Swap
    register_generic!(
        registry,
        InstrumentType::InflationSwap,
        crate::instruments::InflationSwap
    );

    // YoY Inflation Swap
    register_generic!(
        registry,
        InstrumentType::YoYInflationSwap,
        crate::instruments::rates::inflation_swap::YoYInflationSwap
    );

    // Inflation Cap/Floor
    registry.register(
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::default(),
    )?;
    registry.register(
        crate::instruments::rates::inflation_cap_floor::pricer::SimpleInflationCapFloorPricer::with_model(
            ModelKey::Normal,
        ),
    )?;
    Ok(())
}
