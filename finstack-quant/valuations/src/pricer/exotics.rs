//! Pricer registrations for exotic instruments.
//!
//! Covers: Basket, AsianOption, BarrierOption, LookbackOption, QuantoOption,
//! Autocallable, CmsOption, CmsSwap, CliquetOption, RangeAccrual, BermudanSwaption.

use super::{register_generic, InstrumentType, PricerRegistry};

/// Register pricers for exotic instruments (barriers, lookbacks, Asians,
/// autocallables, quantos, cliquets, range accruals, Bermudan swaptions).
pub(crate) fn register_exotic_pricers(
    registry: &mut PricerRegistry,
) -> std::result::Result<(), crate::pricer::PricingError> {
    register_generic!(
        registry,
        InstrumentType::Composite,
        crate::instruments::CompositeInstrument
    );

    register_generic!(
        registry,
        InstrumentType::Basket,
        crate::instruments::exotics::basket::Basket
    );

    // Asian Option

    registry.register(
        crate::instruments::exotics::asian_option::pricer::AsianOptionMcPricer::default(),
    )?;
    registry.register(
        crate::instruments::exotics::asian_option::pricer::AsianOptionAnalyticalGeometricPricer,
    )?;
    registry.register(
        crate::instruments::exotics::asian_option::pricer::AsianOptionSemiAnalyticalTwPricer,
    )?;

    // Barrier Option

    registry.register(
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionMcPricer::default(),
    )?;
    registry.register(
        crate::instruments::exotics::barrier_option::pricer::BarrierOptionAnalyticalPricer,
    )?;

    // Lookback Option

    registry.register(
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionMcPricer::default(),
    )?;
    registry.register(
        crate::instruments::exotics::lookback_option::pricer::LookbackOptionAnalyticalPricer,
    )?;

    // Quanto Option
    registry
        .register(crate::instruments::fx::quanto_option::pricer::QuantoOptionAnalyticalPricer)?;

    registry.register(
        crate::instruments::equity::autocallable::pricer::AutocallableMcPricer::default(),
    )?;

    // CMS Option
    registry.register(crate::instruments::rates::cms_option::pricer::CmsOptionPricer::new())?;

    // CMS Option - Static Replication (Andersen-Piterbarg)
    registry.register(
        crate::instruments::rates::cms_option::replication_pricer::CmsReplicationPricer::new(),
    )?;

    // CMS Swap (first-order Hagan convexity — default)
    registry.register(crate::instruments::rates::cms_swap::pricer::CmsSwapPricer::new())?;

    // CMS Swap - Static Replication (Andersen-Piterbarg; exact smile-aware
    // convexity, preferred for CMS tenors > 10Y or high-vol regimes)
    registry
        .register(crate::instruments::rates::cms_swap::pricer::CmsSwapReplicationPricer::new())?;

    // CMS Spread Option - Gaussian copula with SABR marginals
    registry
        .register(crate::instruments::rates::cms_spread_option::CmsSpreadOptionPricer::new())?;

    // Cliquet Option

    registry.register(
        crate::instruments::equity::cliquet_option::pricer::CliquetOptionMcPricer::default(),
    )?;

    // Range Accrual

    registry.register(
        crate::instruments::exotics::range_accrual::pricer::RangeAccrualStaticReplicationPricer,
    )?;
    registry.register(
        crate::instruments::exotics::range_accrual::pricer::RangeAccrualMcPricer::default(),
    )?;

    // TARN - Hull-White 1F Monte Carlo
    registry.register(crate::instruments::exotics::tarn::TarnPricer::default())?;

    registry.register(crate::instruments::exotics::snowball::SnowballHw1fMcPricer::default())?;
    registry.register(crate::instruments::exotics::snowball::SnowballDiscountingPricer)?;

    // Callable Range Accrual - Hull-White 1F LSMC
    registry.register(
        crate::instruments::exotics::callable_range_accrual::CallableRangeAccrualPricer::default(),
    )?;

    // Bermudan Swaption LSMC (Hull-White 1F Monte Carlo).
    //
    // Callers must supply a complete fitted parameter pair through instrument
    // pricing overrides or market scalars; there is no default-parameter path.

    registry.register(crate::instruments::rates::swaption::BermudanSwaptionPricer::lsmc())?;

    // Bermudan Swaption - Hull-White 1F Tree. See note on the LSMC
    // registration above for the calibration-requirement rationale.
    registry.register(crate::instruments::rates::swaption::BermudanSwaptionPricer::tree())?;

    // Barrier Option - PDE Crank-Nicolson 1D
    registry.register(
        crate::instruments::exotics::barrier_option::pde_pricer::BarrierOptionPdePricer::default(),
    )?;

    // Barrier Option - Monte Carlo Heston

    registry.register(
        crate::instruments::exotics::barrier_option::heston_mc_pricer::BarrierOptionHestonMcPricer::default(),
    )?;

    // Asian Option - Monte Carlo Heston

    registry.register(
        crate::instruments::exotics::asian_option::heston_mc_pricer::AsianOptionHestonMcPricer::default(),
    )?;

    // Bermudan Swaption - LMM Monte Carlo. The loading shape is constructed
    // here, while the required loading scale is supplied explicitly through
    // `model_config.lmm_base_vol`.
    registry.register(
        crate::instruments::rates::swaption::lmm_pricer::BermudanSwaptionLmmPricer::default(),
    )?;

    // Bermudan Swaption - Cheyette Rough Vol Monte Carlo.
    //
    // Registered with `enforce_calibration`: kappa, eta, H (Hurst exponent)
    // and rho are all hardcoded defaults that fully determine the rough-vol
    // smile. Without calibration the resulting price is arbitrary. The guard
    // refuses pricing via the registry so callers are directed to a
    // calibrated model.
    registry.register(
        crate::instruments::rates::swaption::cheyette_rough_pricer::BermudanSwaptionCheyetteRoughPricer::with_config(
            crate::instruments::rates::swaption::cheyette_rough_pricer::CheyetteRoughConfig {
                enforce_calibration: true,
                ..Default::default()
            },
        ),
    )?;

    // Exotic rate products require explicit stochastic or replication models.
    Ok(())
}
