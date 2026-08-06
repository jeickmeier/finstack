//! Temporary scratch harness for validating doc-example market fixtures.

use finstack_quant_core::currency::Currency;
use finstack_quant_core::dates::create_date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::scalars::{MarketScalar, ScalarTimeSeries};
use finstack_quant_core::market_data::surfaces::VolSurface;
use finstack_quant_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_quant_core::money::Money;
use finstack_quant_valuations::instruments::{
    Bond, EquityOption, Instrument, InterestRateSwap, PricingOptions,
};
use finstack_quant_valuations::metrics::MetricId;
use time::macros::date;
use time::Month;

#[test]
fn bond_bucketed_dv01() -> finstack_quant_core::Result<()> {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::example().unwrap();
    let market = MarketContext::new().insert(
        DiscountCurve::builder("USD-TREASURY")
            .base_date(as_of)
            .knots([(0.0, 1.0), (30.0, 0.40)])
            .build()?,
    );
    let metrics = vec![MetricId::BucketedDv01];
    let result = bond.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
    println!("bond pv = {}", result.value.amount());
    println!(
        "dv01 = {:?}",
        result.measures.get(MetricId::BucketedDv01.as_str())
    );
    Ok(())
}

#[test]
fn swap_dv01() -> finstack_quant_core::Result<()> {
    let as_of = date!(2025 - 01 - 01);
    let swap = InterestRateSwap::example_standard()?;
    let market = MarketContext::new()
        .insert(
            DiscountCurve::builder("USD-OIS")
                .base_date(as_of)
                .knots([(0.0, 1.0), (30.0, 0.40)])
                .build()?,
        )
        .insert(
            ForwardCurve::builder("USD-SOFR-3M", 0.25)
                .base_date(as_of)
                .knots([(0.0, 0.04), (30.0, 0.04)])
                .build()?,
        )
        .insert_series(ScalarTimeSeries::new(
            "FIXING:USD-SOFR-3M",
            (0..400)
                .map(|i| (date!(2023 - 12 - 01) + time::Duration::days(i), 0.05))
                .collect::<Vec<_>>(),
            None,
        )?);
    let metrics = vec![MetricId::Dv01];
    let result = swap.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
    println!("swap pv = {}", result.value.amount());
    println!("dv01 = {:?}", result.measures.get(MetricId::Dv01.as_str()));
    Ok(())
}

#[test]
fn option_theta() -> finstack_quant_core::Result<()> {
    let as_of = create_date(2024, Month::January, 1)?;
    let expiry = create_date(2024, Month::July, 1)?;
    let option = EquityOption::european_call(
        "OPT-001",
        "SPX",
        4500.0,
        expiry,
        Money::new(100.0, Currency::USD),
    )?;
    let market = MarketContext::new()
        .insert(
            DiscountCurve::builder("USD-OIS")
                .base_date(as_of)
                .knots([(0.0, 1.0), (5.0, 0.80)])
                .build()?,
        )
        .insert_price("EQUITY-SPOT", MarketScalar::Unitless(4500.0))
        .insert_price("EQUITY-DIVYIELD", MarketScalar::Unitless(0.015))
        .insert_surface(
            VolSurface::builder("EQUITY-VOL")
                .expiries(&[0.25, 0.5, 1.0])
                .strikes(&[4000.0, 4500.0, 5000.0])
                .row(&[0.20, 0.20, 0.20])
                .row(&[0.20, 0.20, 0.20])
                .row(&[0.20, 0.20, 0.20])
                .build()?,
        );
    let metrics = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
        MetricId::Rho,
    ];
    let result = option.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
    println!("option pv = {}", result.value.amount());
    for id in metrics {
        println!("  {} = {:?}", id.as_str(), result.measures.get(id.as_str()));
    }
    Ok(())
}
