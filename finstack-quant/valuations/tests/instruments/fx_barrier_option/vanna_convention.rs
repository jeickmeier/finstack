//! Pins the FX barrier option vanna unit convention.
//!
//! Vanna is reported per **vol point** (0.01 absolute vol) on the σ axis,
//! consistent with Vega, not per unit (decimal) vol. The reference replicates
//! the calculator's own stencil (±1 vol-point parallel surface bump, ±1% spot
//! bump) and normalizes the vol axis in vol points.

use crate::finstack_quant_test_utils::{date, flat_discount_with_tenor, flat_vol_surface};
use finstack_quant_core::currency::Currency;
use finstack_quant_core::market_data::bumps::{
    BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump,
};
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::scalars::MarketScalar;
use finstack_quant_core::money::Money;
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::fx::fx_barrier_option::FxBarrierOption;
use finstack_quant_valuations::instruments::{
    Attributes, BarrierType, Instrument, OptionType, PricingOptions,
};
use finstack_quant_valuations::metrics::MetricId;

fn bump_surface_vol_absolute(
    context: &MarketContext,
    vol_surface_id: &str,
    bump_abs: f64,
) -> finstack_quant_core::Result<MarketContext> {
    context.bump([MarketBump::Curve {
        id: CurveId::from(vol_surface_id),
        spec: BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Fraction,
            value: bump_abs,
            bump_type: BumpType::Parallel,
        },
    }])
}

#[test]
fn vanna_is_reported_per_vol_point() -> finstack_quant_core::Result<()> {
    let as_of = date(2024, 1, 1);
    let expiry = date(2025, 1, 1);
    let spot = 1.10;

    let expiries = [0.25, 0.5, 1.0, 2.0, 5.0];
    let strikes = [0.9, 1.0, 1.1, 1.2, 1.3];
    let market = MarketContext::new()
        .insert(flat_discount_with_tenor("USD-OIS", as_of, 0.03, 5.0))
        .insert(flat_discount_with_tenor("EUR-OIS", as_of, 0.01, 5.0))
        .insert_surface(flat_vol_surface("EURUSD-VOL", &expiries, &strikes, 0.15))
        .insert_price("EURUSD-SPOT", MarketScalar::Unitless(spot));

    // Barrier far from spot so the price surface is smooth around the bumps.
    let option = FxBarrierOption::builder()
        .id(InstrumentId::new("FXBAR-VANNA-CONVENTION"))
        .strike(1.10)
        .barrier(1.60)
        .option_type(OptionType::Call)
        .barrier_type(BarrierType::UpAndOut)
        .expiry(expiry)
        .monitoring_start_date_opt(Some(as_of))
        .notional(Money::new(1_000_000.0, Currency::EUR))
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .use_gobet_miri(false)
        .domestic_discount_curve_id(CurveId::new("USD-OIS"))
        .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
        .fx_spot_id("EURUSD-SPOT".into())
        .vol_surface_id(CurveId::new("EURUSD-VOL"))
        .attributes(Attributes::new())
        .build()
        .expect("FX barrier option should build");

    let result = option.price_with_metrics(
        &market,
        as_of,
        &[MetricId::Vanna],
        PricingOptions::default(),
    )?;
    let vanna = result
        .measures
        .get(&MetricId::Vanna)
        .copied()
        .expect("Vanna in measures");

    let spot_bump_pct = 0.01;
    let vol_bump_abs = 0.01;

    let delta_at = |vol_bump: f64| -> finstack_quant_core::Result<f64> {
        let vol_market = bump_surface_vol_absolute(&market, "EURUSD-VOL", vol_bump)?;
        let up = vol_market.clone().insert_price(
            "EURUSD-SPOT",
            MarketScalar::Unitless(spot * (1.0 + spot_bump_pct)),
        );
        let dn = vol_market.insert_price(
            "EURUSD-SPOT",
            MarketScalar::Unitless(spot * (1.0 - spot_bump_pct)),
        );
        let pv_up = option.value(&up, as_of)?.amount();
        let pv_dn = option.value(&dn, as_of)?.amount();
        Ok((pv_up - pv_dn) / (2.0 * spot * spot_bump_pct))
    };

    let delta_up = delta_at(vol_bump_abs)?;
    let delta_dn = delta_at(-vol_bump_abs)?;
    // Vol-axis width expressed in vol points: 2 × 0.01 × 100 = 2.
    let vanna_ref = (delta_up - delta_dn) / (2.0 * vol_bump_abs * 100.0);

    assert!(
        (vanna - vanna_ref).abs() <= 1e-9 * vanna_ref.abs().max(1.0),
        "FX barrier vanna must be per vol point: expected {vanna_ref}, got {vanna}"
    );
    Ok(())
}
