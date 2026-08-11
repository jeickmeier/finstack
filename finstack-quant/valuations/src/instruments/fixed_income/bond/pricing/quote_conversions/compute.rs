use super::spread_price::{
    par_swap_rate_from_discount, price_from_asw_market, price_from_dm, price_from_oas,
    price_from_z_spread,
};
use super::types::{BondQuoteInput, BondQuoteSet};
use super::yield_price::price_from_ytm;
use crate::constants::numerical::ZERO_TOLERANCE;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fixed_income::bond::pricing::settlement::QuoteDateContext;
use crate::instruments::fixed_income::bond::Bond;
use crate::metrics::{standard_registry, MetricContext, MetricId, MetricRegistry};
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::Result;
use std::sync::Arc;

/// Clear all price-driving market-quote overrides on a bond so downstream
/// pricing calls evaluate the model PV. Used by inversion helpers that need
/// the raw model response even when the bond carries a quoted price override.
pub(crate) fn clear_price_driving_overrides(bond: &mut Bond) {
    let quotes = &mut bond.instrument_pricing_overrides.market_quotes;
    quotes.quoted_clean_price = None;
    quotes.quoted_dirty_price_currency = None;
    quotes.quoted_ytm = None;
    quotes.quoted_ytw = None;
    quotes.quoted_z_spread = None;
    quotes.quoted_oas = None;
    quotes.quoted_discount_margin = None;
    quotes.quoted_i_spread = None;
    quotes.quoted_asw_market = None;
}

/// Convert between price, yield, and spread metrics for a bond.
///
/// The engine:
/// - Normalizes the chosen `quote_input` into a **canonical dirty price in currency**.
/// - Derives the corresponding clean price (% of par) and stamps it into
///   `pricing_overrides.quoted_clean_price` on an internal bond clone.
/// - Uses the standard metrics registry to compute the remaining metrics.
///
/// # Arguments
///
/// * `bond` - Bond to normalize and value. The function clones it before
///   applying the derived clean-price override, so the caller's instance is
///   unchanged.
/// * `curves` - Market context supplying the bond schedule, discount curves,
///   forward curves, and other metric dependencies.
/// * `as_of` - Valuation or trade date from which settlement-aware accrued
///   interest and clean/dirty conversion are determined.
/// * `quote_input` - One observed clean/dirty price, yield, or spread quote
///   used to seed the internally consistent quote set.
///
/// # Returns
///
/// A `BondQuoteSet` containing all computed price, yield, and spread metrics.
///
/// # Errors
///
/// Returns `Err` when:
/// - Market curves are missing
/// - Cashflow schedule building fails
/// - Metric calculations fail
///
/// # Examples
///
/// ```
/// use finstack_quant_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_quant_valuations::instruments::fixed_income::bond::pricing::quote_conversions::{compute_quotes, BondQuoteInput};
/// use finstack_quant_core::market_data::context::MarketContext;
/// use finstack_quant_core::dates::Date;
///
/// # let bond = Bond::example().unwrap();
/// # let curves = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// let quotes = compute_quotes(&bond, &curves, as_of, BondQuoteInput::CleanPricePct(98.5))?;
/// // quotes contains YTM, Z-spread, OAS, etc.
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn compute_quotes(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    quote_input: BondQuoteInput,
) -> Result<BondQuoteSet> {
    // Work on a local clone so we never mutate the caller's bond instance.
    let mut bond_for_metrics = bond.clone();

    // Quote normalization (clean/dirty conversion) must use accrued at quote/settlement date.
    let quote_ctx = QuoteDateContext::new(&bond_for_metrics, curves, as_of)?;
    let accrued_currency = quote_ctx.accrued_at_quote_date;

    let notional = bond_for_metrics.notional.amount();
    if notional.abs() < ZERO_TOLERANCE {
        return Ok(BondQuoteSet {
            clean_price_currency: 0.0,
            clean_price_pct: 0.0,
            dirty_price_currency: 0.0,
            ytm: None,
            ytw: None,
            z_spread: None,
            discount_margin: None,
            oas: None,
            asw_par: None,
            asw_market: None,
            i_spread: None,
        });
    }

    // 1) Stamp the quote input into the corresponding price-driving override
    //    on the bond clone, then delegate to `base_value` (which runs the same
    //    precedence chain used by the pricing pipeline). This keeps
    //    `compute_quotes` and `Bond::base_value` in lock-step and eliminates
    //    the per-variant inversion logic that used to live here.
    clear_price_driving_overrides(&mut bond_for_metrics);
    {
        let quotes = &mut bond_for_metrics.instrument_pricing_overrides.market_quotes;
        match quote_input {
            BondQuoteInput::CleanPricePct(v) => quotes.quoted_clean_price = Some(v),
            BondQuoteInput::DirtyPriceCurrency(v) => quotes.quoted_dirty_price_currency = Some(v),
            BondQuoteInput::Ytm(v) => quotes.quoted_ytm = Some(v),
            BondQuoteInput::Ytw(v) => quotes.quoted_ytw = Some(v),
            BondQuoteInput::ZSpread(v) => quotes.quoted_z_spread = Some(v),
            BondQuoteInput::DiscountMargin(v) => quotes.quoted_discount_margin = Some(v),
            BondQuoteInput::Oas(v) => quotes.quoted_oas = Some(v),
            BondQuoteInput::AswMarket(v) => quotes.quoted_asw_market = Some(v),
            BondQuoteInput::ISpread(v) => quotes.quoted_i_spread = Some(v),
        }
    }

    let base_value = bond_for_metrics.value(curves, as_of)?;
    let dirty_price_currency = base_value.amount();
    let clean_price_currency = dirty_price_currency - accrued_currency;
    let clean_price_pct = clean_price_currency / notional * 100.0;

    // Stamp the canonical clean price quote into pricing_overrides so that all
    // existing metric calculators interpret this as the market price.
    // (Replaces the specific quote field with the clean-price normalization
    // expected by the downstream metric calculators.)
    clear_price_driving_overrides(&mut bond_for_metrics);
    bond_for_metrics
        .instrument_pricing_overrides
        .market_quotes
        .quoted_clean_price = Some(clean_price_pct);

    // 2) Build metric context and use the standard registry for the rest.
    let base_value = bond_for_metrics.value(curves, as_of)?;
    let registry: MetricRegistry = standard_registry().clone();

    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond_for_metrics.clone());
    let curves_arc = Arc::new(curves.clone());
    let mut ctx = MetricContext::new(
        instrument_arc,
        curves_arc,
        as_of,
        base_value,
        MetricContext::default_config(),
    );
    ctx.notional = Some(bond_for_metrics.notional);

    // Pre-populate accrued since we've already computed it.
    ctx.computed.insert(MetricId::Accrued, accrued_currency);

    // Request the core price/yield/spread metrics.
    let metric_ids = [
        MetricId::Ytm,
        MetricId::Ytw,
        MetricId::ZSpread,
        MetricId::DiscountMargin,
        MetricId::Oas,
        MetricId::ASWPar,
        MetricId::ASWMarket,
        MetricId::ISpread,
    ];

    // Some quote metrics are not applicable to all bond types (e.g. FRN vs fixed),
    // and we want `compute_quotes` to return whatever is available rather than
    // failing the entire quote set.
    for metric_id in &metric_ids {
        if let Err(err) = registry.compute(std::slice::from_ref(metric_id), &mut ctx) {
            tracing::debug!(
                metric_id = metric_id.as_str(),
                error = %err,
                "Bond quote engine metric computation failed; leaving unset"
            );
        }
    }

    // Read back the metrics we care about.
    let ytm = ctx.computed.get(&MetricId::Ytm).copied();
    let ytw = ctx.computed.get(&MetricId::Ytw).copied();
    let z_spread = ctx.computed.get(&MetricId::ZSpread).copied();
    let discount_margin = ctx.computed.get(&MetricId::DiscountMargin).copied();
    let oas = ctx.computed.get(&MetricId::Oas).copied();
    let asw_par = ctx.computed.get(&MetricId::ASWPar).copied();
    let asw_market = ctx.computed.get(&MetricId::ASWMarket).copied();
    let i_spread = ctx.computed.get(&MetricId::ISpread).copied();

    Ok(BondQuoteSet {
        clean_price_currency,
        clean_price_pct,
        dirty_price_currency,
        ytm,
        ytw,
        z_spread,
        discount_margin,
        oas,
        asw_par,
        asw_market,
        i_spread,
    })
}

/// Resolve any bond price-quote override into a dirty price in currency units.
///
/// Follows the precedence chain documented on [`MarketQuoteOverrides`]:
///
/// 1. `quoted_dirty_price_currency` → return directly
/// 2. `quoted_clean_price` → convert to dirty using quote-date accrued
/// 3. `quoted_ytm` → [`price_from_ytm`]
/// 4. `quoted_ytw` → [`super::yield_price::price_from_ytw`]
/// 5. `quoted_z_spread` → [`price_from_z_spread`]
/// 6. `quoted_oas` → [`price_from_oas`]
/// 7. `quoted_discount_margin` → [`price_from_dm`]
/// 8. `quoted_i_spread` → par-swap-rate inversion + [`price_from_ytm`]
/// 9. `quoted_asw_market` → ASW market-convention inversion
///
/// Returns `Ok(None)` when no price-driving override is set so the caller can
/// fall through to model pricing.
///
/// [`MarketQuoteOverrides`]: crate::instruments::pricing_overrides::MarketQuoteOverrides
pub(crate) fn price_from_quote_overrides(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
) -> Result<Option<f64>> {
    let quotes = &bond.instrument_pricing_overrides.market_quotes;

    // Fast path: no price-driving override is set.
    if quotes.quoted_dirty_price_currency.is_none()
        && quotes.quoted_clean_price.is_none()
        && quotes.quoted_ytm.is_none()
        && quotes.quoted_ytw.is_none()
        && quotes.quoted_z_spread.is_none()
        && quotes.quoted_oas.is_none()
        && quotes.quoted_discount_margin.is_none()
        && quotes.quoted_i_spread.is_none()
        && quotes.quoted_asw_market.is_none()
    {
        return Ok(None);
    }

    // Dirty-price override: short-circuit, no accrued-interest conversion needed.
    if let Some(dirty) = quotes.quoted_dirty_price_currency {
        return Ok(Some(dirty));
    }

    // All remaining inversions settle the quote at the bond's quote date.
    let quote_ctx = QuoteDateContext::new(bond, curves, as_of)?;
    let accrued_currency = quote_ctx.accrued_at_quote_date;
    let notional = bond.notional.amount();

    if let Some(clean_pct) = quotes.quoted_clean_price {
        return Ok(Some(quote_ctx.dirty_from_clean_pct(clean_pct, notional)));
    }
    if let Some(ytm) = quotes.quoted_ytm {
        let flows = quote_ctx.entitled_flows(bond, curves, as_of)?;
        return Ok(Some(price_from_ytm(
            bond,
            &flows,
            quote_ctx.quote_date,
            ytm,
        )?));
    }
    if let Some(ytw) = quotes.quoted_ytw {
        // For non-callable bonds, YTW == YTM, and the inversion is identical.
        // For callable bonds, the quote-override path uses maturity flows
        // (consistent with `quoted_ytm`); users who need exercise-aware
        // pricing should use `quoted_oas` instead.
        let flows = quote_ctx.entitled_flows(bond, curves, as_of)?;
        return Ok(Some(price_from_ytm(
            bond,
            &flows,
            quote_ctx.quote_date,
            ytw,
        )?));
    }
    if let Some(z) = quotes.quoted_z_spread {
        // `price_from_z_spread` derives the settlement (`quote_date`) origin
        // internally, so it takes the valuation `as_of` here.
        return Ok(Some(price_from_z_spread(bond, curves, as_of, z)?));
    }
    if let Some(oas) = quotes.quoted_oas {
        return Ok(Some(price_from_oas(
            bond,
            curves,
            quote_ctx.quote_date,
            oas,
        )?));
    }
    if let Some(dm) = quotes.quoted_discount_margin {
        return Ok(Some(price_from_dm(bond, curves, quote_ctx.quote_date, dm)?));
    }
    if let Some(i_spread) = quotes.quoted_i_spread {
        let par_swap_rate = par_swap_rate_from_discount(bond, curves, quote_ctx.quote_date)?;
        let ytm = i_spread + par_swap_rate;
        let flows = quote_ctx.entitled_flows(bond, curves, as_of)?;
        return Ok(Some(price_from_ytm(
            bond,
            &flows,
            quote_ctx.quote_date,
            ytm,
        )?));
    }
    if let Some(asw) = quotes.quoted_asw_market {
        return Ok(Some(price_from_asw_market(
            bond,
            curves,
            quote_ctx.quote_date,
            asw,
        )?));
    }

    // Unreachable: the early-return above guarantees at least one override is set.
    let _ = accrued_currency;
    Ok(None)
}
