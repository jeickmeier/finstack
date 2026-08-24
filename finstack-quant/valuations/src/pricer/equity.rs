//! Pricer registrations for equity instruments.
//!
//! Covers: Equity, EquityFuture, EquityFutureOption, VolatilityIndexFutureOption,
//! EquityTotalReturnFuture, EquityOption,
//! EquityTotalReturnSwap, VarianceSwap,
//! VolatilityIndexFuture,
//! RealEstateAsset, LeveredRealEstateEquity, PrivateMarketsFund.

use super::{register_generic, InstrumentType, ModelKey, PricerRegistry};

/// Register pricers for equity instruments.
pub(crate) fn register_equity_pricers(registry: &mut PricerRegistry) {
    register_generic!(
        registry,
        InstrumentType::EquityFuture,
        crate::instruments::EquityFuture
    );
    register_generic!(
        registry,
        InstrumentType::EquityTotalReturnFuture,
        crate::instruments::EquityTotalReturnFuture
    );
    register_generic!(
        registry,
        InstrumentType::EquityFutureOption,
        crate::instruments::EquityFutureOption
    );
    register_generic!(
        registry,
        InstrumentType::VolatilityIndexFutureOption,
        crate::instruments::VolatilityIndexFutureOption
    );

    register_generic!(
        registry,
        InstrumentType::Equity,
        crate::instruments::equity::Equity
    );

    // Equity Option
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::Black76,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::default(),
    );
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::Discounting,
        crate::instruments::equity::equity_option::pricer::SimpleEquityOptionBlackPricer::with_model(
            ModelKey::Discounting,
        ),
    );

    registry.register(
        InstrumentType::EquityOption,
        ModelKey::HestonFourier,
        crate::instruments::equity::equity_option::pricer::EquityOptionHestonFourierPricer,
    );

    // Equity TRS
    register_generic!(
        registry,
        InstrumentType::EquityTotalReturnSwap,
        crate::instruments::equity::equity_trs::EquityTotalReturnSwap
    );

    // Variance Swap
    register_generic!(
        registry,
        InstrumentType::VarianceSwap,
        crate::instruments::equity::variance_swap::VarianceSwap
    );

    // Volatility Index Future
    register_generic!(
        registry,
        InstrumentType::VolatilityIndexFuture,
        crate::instruments::equity::vol_index_future::VolatilityIndexFuture
    );

    // Corporate discounted cash flow.
    registry.register(
        InstrumentType::Dcf,
        ModelKey::Discounting,
        crate::instruments::equity::dcf_equity::pricer::DcfPricer,
    );

    // Real Estate Asset - uses GenericInstrumentPricer (curve dependencies)
    register_generic!(
        registry,
        InstrumentType::RealEstateAsset,
        crate::instruments::RealEstateAsset
    );

    // Levered Real Estate Equity - uses GenericInstrumentPricer
    register_generic!(
        registry,
        InstrumentType::LeveredRealEstateEquity,
        crate::instruments::LeveredRealEstateEquity
    );

    // Private Markets Fund - uses GenericInstrumentPricer.
    register_generic!(
        registry,
        InstrumentType::PrivateMarketsFund,
        crate::instruments::equity::pe_fund::PrivateMarketsFund
    );

    // Equity Option - PDE Crank-Nicolson 1D (Black-Scholes)
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::PdeCrankNicolson1D,
        crate::instruments::equity::equity_option::pde_pricer::EquityOptionPdePricer::default(),
    );

    // Equity Option - PDE ADI 2D (Heston)
    registry.register(
        InstrumentType::EquityOption,
        ModelKey::PdeAdi2D,
        crate::instruments::equity::equity_option::pde2d_pricer::EquityOptionHestonPdePricer::default(),
    );

    // Equity Option - Monte Carlo Heston

    registry.register(
        InstrumentType::EquityOption,
        ModelKey::MonteCarloHeston,
        crate::instruments::equity::equity_option::heston_mc_pricer::EquityOptionHestonMcPricer::default(),
    );

    // Equity Option - Rough Heston Fourier

    registry.register(
        InstrumentType::EquityOption,
        ModelKey::RoughHestonFourier,
        crate::instruments::equity::equity_option::rough_heston_fourier_pricer::EquityOptionRoughHestonFourierPricer,
    );

    // Equity Option - Monte Carlo Rough Heston

    registry.register(
        InstrumentType::EquityOption,
        ModelKey::MonteCarloRoughHeston,
        crate::instruments::equity::equity_option::rough_heston_mc_pricer::EquityOptionRoughHestonMcPricer::default(),
    );

    // Equity Option - Monte Carlo Rough Bergomi

    registry.register(
        InstrumentType::EquityOption,
        ModelKey::MonteCarloRoughBergomi,
        crate::instruments::equity::equity_option::rough_bergomi_mc_pricer::EquityOptionRoughBergomiMcPricer::default(),
    );
}
