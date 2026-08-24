//! Pricer infrastructure: type-safe pricing dispatch via registry pattern.
//!
//! This module provides a registry-based pricing system that maps
//! (instrument type, model) pairs to specific pricer implementations.
//! The system uses enum-based dispatch for type safety rather than string
//! comparisons.
//!
//! # Module structure
//!
//! Core types are split into focused submodules:
//! - `keys`: [`crate::pricer::InstrumentType`], [`crate::pricer::ModelKey`],
//!   [`crate::pricer::PricerKey`]
//! - `errors`: [`crate::pricer::PricingError`],
//!   [`crate::pricer::PricingErrorContext`]
//! - `registry`: [`crate::pricer::Pricer`], [`crate::pricer::PricerRegistry`],
//!   `expect_inst`
//!
//! The registration logic is split into asset-class submodules:
//! - `rates`: Bond, IRS, FRA, BasisSwap, Deposit, CapFloor, Swaption, Repo, DCF, IR futures/options
//! - `credit`: CDS, CDSIndex, CDSTranche, CDSOption, StructuredCredit
//! - `equity`: Equity, EquityFuture, EquityFutureOption, EquityTotalReturnFuture, EquityOption, EquityTRS, VarianceSwap, VolatilityIndexFuture, RealEstate, PE fund
//! - `fx`: FxSpot, FxFuture, FxFutureOption, FxSwap, XccySwap, FxOption, FxVarianceSwap, FxForward, NDF, FX barrier/digital/touch
//! - `fixed_income`: FIIndexTRS, Convertible, InflationLinkedBond, RevolvingCredit, TermLoan, MBS, TBA, CMO
//! - `inflation`: InflationSwap, YoYInflationSwap, InflationCapFloor
//! - `exotics`: Basket, Asian, Barrier, Lookback, Quanto, Autocallable, CMS, Cliquet, RangeAccrual, BermudanSwaption
//! - `commodity`: CommodityFuture, CommodityFutureOption, CommodityForward, CommoditySwap, CommodityOption, CommoditySwaption, CommoditySpreadOption

// Core submodules
mod enrichment;
mod errors;
pub mod json;
mod keys;
mod registry;
pub mod structured_credit_json;

pub use errors::{PricingError, PricingErrorContext};
pub use json::{
    instrument_envelope_from_spec, list_models, list_models_grouped, list_standard_metrics,
    list_standard_metrics_grouped, metric_value_from_instrument_json, parse_boxed_instrument_json,
    parse_instrument_json, parse_model_key, present_metric_values_from_instrument_json,
    present_standard_option_greeks_from_instrument_json, pretty_instrument_json,
    price_instrument_json, validate_instrument_json, validate_typed_instrument_json,
    STANDARD_OPTION_GREEKS,
};
pub use keys::{InstrumentType, ModelKey, PricerKey};
pub use registry::{expect_inst, Pricer, PricerRegistry, PricingDispatch};
pub use structured_credit_json::{
    structured_credit_tranche_breakeven_cdr_json, structured_credit_tranche_discount_margin_json,
    structured_credit_tranche_metrics_json, structured_credit_tranche_oas_json,
    structured_credit_tranche_scenario_table_json,
};

// Fourier pricing via the Fang-Oosterlee (2008) COS method.
//
// A Lewis (2001) pricer was previously exposed alongside COS but was known-
// divergent off-ATM and silently dropped non-finite integrand panels behind a
// `max(0.0)` clamp. It was removed; COS is the only Fourier method finstack
// exposes, so the implementation lives directly in `pricer::cos` rather than
// inside a single-member `pricer::fourier` namespace.
pub mod cos;

// Asset-class registration submodules
mod commodity;
mod credit;
mod equity;
mod exotics;
mod fixed_income;
mod fx;
mod inflation;
mod rates;

use std::sync::{Arc, OnceLock};

/// Register a [`GenericInstrumentPricer`](crate::instruments::common_impl::GenericInstrumentPricer)
/// for an instrument type, collapsing the repetitive registration boilerplate
/// shared by every asset-class shard.
///
/// Two forms are supported:
///
/// - `register_generic!(registry, InstrumentType::Foo, crate::instruments::FooType);`
///   registers the discounting pricer (`GenericInstrumentPricer::<T>::discounting`)
///   under `ModelKey::Discounting`.
/// - `register_generic!(registry, InstrumentType::Foo, crate::instruments::FooType, ModelKey::Bar);`
///   registers `GenericInstrumentPricer::<T>::new(InstrumentType::Foo, ModelKey::Bar)`
///   under the explicit model key.
///
/// Both forms expand to a single `registry.register(...)` call with behavior
/// byte-identical to the hand-written registrations they replace.
macro_rules! register_generic {
    ($registry:expr, $inst:expr, $ty:ty $(,)?) => {
        $registry.register(
            $inst,
            $crate::pricer::ModelKey::Discounting,
            $crate::instruments::common_impl::GenericInstrumentPricer::<$ty>::discounting($inst),
        )?
    };
    ($registry:expr, $inst:expr, $ty:ty, $model:expr $(,)?) => {
        $registry.register(
            $inst,
            $model,
            $crate::instruments::common_impl::GenericInstrumentPricer::<$ty>::new($inst, $model),
        )?
    };
}

pub(crate) use register_generic;

/// Register all standard pricers explicitly.
///
/// This function keeps the full registration list in one visible place while
/// delegating concrete registration tables to the asset-class submodules below.
/// This explicit approach provides better IDE support, easier debugging, and
/// clearer dependency tracking compared to auto-registration.
fn register_all_pricers(registry: &mut PricerRegistry) -> std::result::Result<(), PricingError> {
    rates::register_rates_pricers(registry)?;
    credit::register_credit_pricers(registry)?;
    equity::register_equity_pricers(registry)?;
    fx::register_fx_pricers(registry)?;
    fixed_income::register_fixed_income_pricers(registry)?;
    inflation::register_inflation_pricers(registry)?;
    exotics::register_exotic_pricers(registry)?;
    commodity::register_commodity_pricers(registry)?;
    Ok(())
}

/// Build a standard pricer registry with all registered pricers.
///
/// This helper explicitly registers all instrument pricers into a fresh registry.
/// The explicit registration approach provides better visibility, IDE support, and
/// debugging capabilities compared to the previous auto-registration system.
///
/// All 40+ instrument pricers are registered in the `register_all_pricers` function.
/// Note: All pricers now use standardized parameter ordering: (instrument, market, as_of).
///
/// Duplicate built-in registrations fail immediately at their registration
/// site. External callers receive [`PricingError::DuplicateRegistration`].
fn build_standard_registry() -> std::result::Result<PricerRegistry, PricingError> {
    let mut registry = PricerRegistry::new();
    register_all_pricers(&mut registry)?;
    Ok(registry)
}

static STANDARD_PRICER_REGISTRY: OnceLock<Arc<PricerRegistry>> = OnceLock::new();

/// Return the shared standard pricer registry by reference.
///
/// This is the primary public entry point for accessing the built-in pricer set.
/// Callers that need to mutate a registry should start from `standard_registry().clone()`.
#[allow(clippy::expect_used)]
pub fn standard_registry() -> &'static PricerRegistry {
    STANDARD_PRICER_REGISTRY
        .get_or_init(|| {
            Arc::new(
                build_standard_registry().expect("built-in pricer registrations must be valid"),
            )
        })
        .as_ref()
}

/// Return the shared standard pricer registry.
///
/// The registry is initialized once and then cloned via `Arc` for cheap reuse
/// across instrument-side pricing calls.
#[allow(clippy::expect_used)]
pub(crate) fn shared_standard_registry() -> Arc<PricerRegistry> {
    Arc::clone(STANDARD_PRICER_REGISTRY.get_or_init(|| {
        Arc::new(build_standard_registry().expect("built-in pricer registrations must be valid"))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::ValuationResult;
    use finstack_quant_core::currency::Currency;
    use finstack_quant_core::money::Money;
    use std::ptr;

    struct DummyPricer;

    impl Pricer for DummyPricer {
        fn key(&self) -> PricerKey {
            PricerKey::new(InstrumentType::Deposit, ModelKey::Tree)
        }

        fn price_dyn(
            &self,
            _instrument: &dyn crate::instruments::Instrument,
            _market: &finstack_quant_core::market_data::MarketContext,
            as_of: finstack_quant_core::dates::Date,
        ) -> std::result::Result<ValuationResult, PricingError> {
            Ok(ValuationResult::stamped(
                "dummy",
                as_of,
                Money::new(0.0, Currency::USD),
            ))
        }
    }

    /// `register` returns a typed collision error without overwriting.
    #[test]
    fn register_rejects_duplicate_key() {
        let mut registry = PricerRegistry::new();
        let key = PricerKey::new(InstrumentType::Deposit, ModelKey::Tree);

        registry
            .register(InstrumentType::Deposit, ModelKey::Tree, DummyPricer)
            .expect("first registration");
        let error = registry
            .register(InstrumentType::Deposit, ModelKey::Tree, DummyPricer)
            .expect_err("duplicate registration must fail");
        assert_eq!(error, PricingError::DuplicateRegistration { key });
        assert!(registry.get_pricer(key).is_some());
    }

    #[test]
    fn replace_is_the_explicit_overwrite_operation() {
        let mut registry = PricerRegistry::new();
        let key = PricerKey::new(InstrumentType::Deposit, ModelKey::Tree);
        registry
            .register(InstrumentType::Deposit, ModelKey::Tree, DummyPricer)
            .expect("first registration");
        registry.replace(InstrumentType::Deposit, ModelKey::Tree, DummyPricer);
        assert!(registry.get_pricer(key).is_some());
    }

    #[test]
    fn standard_registry_returns_shared_singleton() {
        assert!(ptr::eq(standard_registry(), standard_registry()));
        assert!(ptr::eq(
            standard_registry(),
            shared_standard_registry().as_ref(),
        ));
    }

    #[test]
    fn cloned_standard_registry_is_independently_mutable() {
        let key = PricerKey::new(InstrumentType::Deposit, ModelKey::Tree);
        assert!(standard_registry().get_pricer(key).is_none());

        let mut cloned = standard_registry().clone();
        cloned
            .register(InstrumentType::Deposit, ModelKey::Tree, DummyPricer)
            .expect("new clone registration");

        assert!(cloned.get_pricer(key).is_some());
        assert!(standard_registry().get_pricer(key).is_none());
    }

    #[test]
    fn standard_registry_exposes_range_accrual_analytic_and_mc_models() {
        let registry = standard_registry();
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::RangeAccrual,
                ModelKey::StaticReplication,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::RangeAccrual,
                ModelKey::MonteCarloGBM,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::Tarn,
                ModelKey::MonteCarloHullWhite1F,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::Snowball,
                ModelKey::MonteCarloHullWhite1F,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::Snowball,
                ModelKey::Discounting,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::CallableRangeAccrual,
                ModelKey::MonteCarloHullWhite1F,
            ))
            .is_some());
        assert!(registry
            .get_pricer(PricerKey::new(
                InstrumentType::CmsSpreadOption,
                ModelKey::StaticReplication,
            ))
            .is_some());
    }
}
