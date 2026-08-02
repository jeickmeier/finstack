use finstack_quant_core::{
    dates::Date,
    money::Money,
    types::{CurveId, InstrumentId},
};
use finstack_quant_valuations::constants::isda::STANDARD_RECOVERY_SENIOR;
use finstack_quant_valuations::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive, PremiumLegSpec, ProtectionLegSpec,
};
use finstack_quant_valuations::instruments::{Attributes, InstrumentPricingOverrides};
use rust_decimal::Decimal;

/// Create a CDS buy protection position using the builder pattern.
#[allow(clippy::too_many_arguments)]
pub fn cds_buy_protection(
    id: impl Into<InstrumentId>,
    notional: Money,
    spread_bp: f64,
    start: Date,
    maturity: Date,
    discount_curve_id: impl Into<CurveId>,
    credit_id: impl Into<CurveId>,
) -> finstack_quant_core::Result<CreditDefaultSwap> {
    let convention = CDSConvention::IsdaNa;
    let day_count = convention.day_count();
    let frequency = convention.frequency();
    let business_day_convention = convention.business_day_convention();
    let stub = convention.stub_convention();

    let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
        finstack_quant_core::Error::Validation(format!(
            "spread_bp {} cannot be represented as Decimal: {}",
            spread_bp, e
        ))
    })?;

    let cds = CreditDefaultSwap::builder()
        .id(id.into())
        .notional(notional)
        .side(PayReceive::Pay)
        .convention(convention)
        .premium(PremiumLegSpec {
            start,
            end: maturity,
            frequency,
            stub,
            business_day_convention,
            calendar_id: Some(convention.default_calendar().to_string()),
            day_count,
            spread_bp: spread_bp_decimal,
            discount_curve_id: discount_curve_id.into(),
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: credit_id.into(),
            recovery_rate: STANDARD_RECOVERY_SENIOR,
            settlement_delay: convention.settlement_delay(),
        })
        .instrument_pricing_overrides(InstrumentPricingOverrides::default())
        .attributes(Attributes::new())
        .build()?;

    cds.validate()?;
    Ok(cds)
}

/// Create a CDS sell protection position using the builder pattern.
#[allow(clippy::too_many_arguments)]
pub fn cds_sell_protection(
    id: impl Into<InstrumentId>,
    notional: Money,
    spread_bp: f64,
    start: Date,
    maturity: Date,
    discount_curve_id: impl Into<CurveId>,
    credit_id: impl Into<CurveId>,
) -> finstack_quant_core::Result<CreditDefaultSwap> {
    let convention = CDSConvention::IsdaNa;
    let day_count = convention.day_count();
    let frequency = convention.frequency();
    let business_day_convention = convention.business_day_convention();
    let stub = convention.stub_convention();

    let spread_bp_decimal = Decimal::try_from(spread_bp).map_err(|e| {
        finstack_quant_core::Error::Validation(format!(
            "spread_bp {} cannot be represented as Decimal: {}",
            spread_bp, e
        ))
    })?;

    let cds = CreditDefaultSwap::builder()
        .id(id.into())
        .notional(notional)
        .side(PayReceive::Receive)
        .convention(convention)
        .premium(PremiumLegSpec {
            start,
            end: maturity,
            frequency,
            stub,
            business_day_convention,
            calendar_id: Some(convention.default_calendar().to_string()),
            day_count,
            spread_bp: spread_bp_decimal,
            discount_curve_id: discount_curve_id.into(),
        })
        .protection(ProtectionLegSpec {
            credit_curve_id: credit_id.into(),
            recovery_rate: STANDARD_RECOVERY_SENIOR,
            settlement_delay: convention.settlement_delay(),
        })
        .instrument_pricing_overrides(InstrumentPricingOverrides::default())
        .attributes(Attributes::new())
        .build()?;

    cds.validate()?;
    Ok(cds)
}
