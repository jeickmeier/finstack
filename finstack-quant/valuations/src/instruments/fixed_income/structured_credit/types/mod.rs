//! Type definitions for structured credit instruments.
//!
//! This module contains all data structures for structured credit instruments:
//! - `StructuredCredit` - The main instrument type
//! - AssetPool and asset types
//! - Tranche structure and coupon types
//! - Waterfall distribution types
//! - Behavioral model specifications
//! - Result types for valuation

pub(crate) mod constants;
pub(crate) mod enums;
pub(crate) mod pool;
/// SoA layout for pool assets.
pub(crate) mod pool_state;
pub(crate) mod results;
pub(crate) mod setup;
pub(crate) mod tranches;
pub(crate) mod waterfall;

mod constructors;
mod instrument;
mod pricing_methods;
mod reinvestment;
mod stochastic;
mod structured_credit_impl;

pub use enums::TrancheSeniority;
pub use enums::{AssetType, DealType, PaymentMode, TriggerConsequence};

pub use pool::AssetPool;
pub use pool::{
    calculate_pool_stats, ConcentrationCheckResult, ConcentrationViolation, PoolAsset, PoolStats,
    ReinvestmentCriteria, ReinvestmentPeriod, RepLine,
};
pub(crate) use pool_state::PoolState;

pub use tranches::{
    CoverageTrigger, CreditEnhancement, Tranche, TrancheBehaviorType, TrancheBuilder,
    TrancheCoupon, TrancheStructure,
};

pub use setup::{CoverageTestConfig, DealConfig, DealDates, DealFees, DefaultAssumptions};

pub use reinvestment::ReinvestmentManager;

pub(crate) use waterfall::DiversionRecord;
pub use waterfall::{
    AfcSpec, AllocationMode, ControlledAccumulationSpec, CoverageTestType, EarlyAmortizationSpec,
    ExcessSpreadSpec, ManagementFeeType, PaymentCalculation, PaymentRecord, PaymentType, Recipient,
    RecipientType, RoundingConvention, ShiftingInterestSpec, ShiftingInterestStep, StepDownSpec,
    StepDownTrigger, Waterfall, WaterfallBuilder, WaterfallDistribution, WaterfallRules,
    WaterfallTier, WaterfallWorkspace,
};

pub use results::{TrancheCashflows, TrancheValuation};

pub use crate::instruments::fixed_income::structured_credit::pricing::{
    CorrelationStructure, StochasticDefaultSpec, StochasticPrepaySpec,
};

pub use crate::cashflow::builder::{DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec};

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::rates::irs::InterestRateSwap;
use finstack_quant_core::dates::{BusinessDayConvention, Date, Tenor};
use finstack_quant_core::money::Money;
use finstack_quant_core::types::{CurveId, InstrumentId};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Market conditions that affect prepayment behavior.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketConditions {
    /// Current refinancing rate.
    pub refi_rate: f64,
    /// Rate at origination for refinancing incentive calculation.
    pub original_rate: Option<f64>,
    /// Home price appreciation (for mortgages).
    pub hpa: Option<f64>,
    /// Unemployment rate.
    pub unemployment: Option<f64>,
    /// Seasonal adjustment factor.
    pub seasonal_factor: Option<f64>,
    /// Custom market factors.
    pub custom_factors: IndexMap<String, f64>,
}

/// Credit factors affecting default probability.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditFactors {
    /// Current FICO/credit score.
    pub credit_score: Option<u32>,
    /// Debt-to-income ratio.
    pub dti: Option<f64>,
    /// Loan-to-value ratio.
    pub ltv: Option<f64>,
    /// Payment delinquency status (days).
    pub delinquency_days: u32,
    /// Unemployment rate.
    pub unemployment_rate: Option<f64>,
    /// Annual net operating income for CMBS collateral, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annual_noi: Option<Money>,
    /// Annual debt service for CMBS collateral, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annual_debt_service: Option<Money>,
    /// Additional custom factors.
    pub custom_factors: IndexMap<String, f64>,
}

/// Deal metadata (counterparties and identifiers).
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    /// Manager identifier (for CLO).
    pub manager_id: Option<String>,
    /// Servicer identifier (for ABS/RMBS/CMBS).
    pub servicer_id: Option<String>,
    /// Master servicer identifier (for CMBS/RMBS).
    pub master_servicer_id: Option<String>,
    /// Special servicer identifier (for CMBS).
    pub special_servicer_id: Option<String>,
    /// Trustee identifier (for ABS).
    pub trustee_id: Option<String>,
}

/// Behavioral overrides for prepayment, default, and recovery assumptions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    /// Override prepayment with constant annual CPR.
    pub cpr_annual: Option<f64>,
    /// Override prepayment with monthly ABS speed.
    pub abs_speed: Option<f64>,
    /// Override prepayment with PSA multiplier.
    pub psa_speed_multiplier: Option<f64>,
    /// Override default with constant annual CDR.
    pub cdr_annual: Option<f64>,
    /// Override default with SDA multiplier.
    pub sda_speed_multiplier: Option<f64>,
    /// Override recovery with constant rate.
    pub recovery_rate: Option<f64>,
    /// Override recovery lag (months).
    pub recovery_lag_months: Option<u32>,
    /// Reinvestment price constraint (% of par).
    pub reinvestment_price: Option<f64>,
}

/// Configuration for deterministic + optional stochastic credit behavior models.
///
/// This groups the "credit model" knobs that were previously exposed as many
/// top-level fields on [`StructuredCredit`]. The struct is intended to be
/// embedded via `#[serde(flatten)]` to preserve the existing JSON shape.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CreditModelConfig {
    /// Prepayment model specification.
    #[serde(default = "CreditModelConfig::default_prepayment_spec")]
    pub prepayment_spec: PrepaymentModelSpec,

    /// Default model specification.
    #[serde(default = "CreditModelConfig::default_default_spec")]
    pub default_spec: DefaultModelSpec,

    /// Recovery model specification.
    #[serde(default = "CreditModelConfig::default_recovery_spec")]
    pub recovery_spec: RecoveryModelSpec,

    /// Optional stochastic prepayment model specification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stochastic_prepay_spec: Option<StochasticPrepaySpec>,

    /// Optional stochastic default model specification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stochastic_default_spec: Option<StochasticDefaultSpec>,

    /// Optional correlation structure for stochastic modeling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_structure: Option<CorrelationStructure>,
}

/// Unified structured credit instrument representation.
///
/// This single type handles CLO, ABS, CMBS, and RMBS instruments using
/// composition for deal-specific differences.
#[derive(
    Clone,
    finstack_quant_valuations_macros::FinancialBuilder,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct StructuredCredit {
    /// Unique instrument identifier.
    pub id: InstrumentId,

    /// Deal classification (ABS/CLO/CMBS/RMBS).
    pub deal_type: DealType,

    /// Asset pool definition.
    pub pool: AssetPool,

    /// Tranche structure.
    pub tranches: TrancheStructure,

    /// Key dates.
    /// Deal closing date (issuance).
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub closing_date: Date,
    /// First payment date to tranches.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub first_payment_date: Date,
    /// End of reinvestment period (if applicable).
    #[serde(default, with = "finstack_quant_core::wire::optional_date")]
    #[schemars(with = "Option<finstack_quant_core::wire::DateWire>")]
    pub reinvestment_end_date: Option<Date>,
    /// Legal final maturity date.
    #[serde(with = "finstack_quant_core::wire::date")]
    #[schemars(with = "finstack_quant_core::wire::DateWire")]
    pub maturity: Date,

    /// Payment frequency for the structure.
    pub frequency: Tenor,

    /// Optional payment calendar identifier for schedule adjustments.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_calendar_id: Option<String>,

    /// Business day convention for tranche payments (defaults to Following).
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_business_day_convention: Option<BusinessDayConvention>,

    /// Discount curve for valuation.
    pub discount_curve_id: CurveId,

    /// Attributes for scenario selection.
    #[builder(default)]
    /// Instrument-owned pricing inputs.
    #[serde(
        default,
        skip_serializing_if = "crate::instruments::InstrumentPricingOverrides::is_empty"
    )]
    pub instrument_pricing_overrides: crate::instruments::InstrumentPricingOverrides,
    /// Metric-time pricing configuration.
    #[builder(default)]
    #[serde(
        default,
        skip_serializing_if = "crate::instruments::MetricPricingOverrides::is_empty"
    )]
    pub metric_pricing_overrides: crate::instruments::MetricPricingOverrides,
    /// Scenario-only pricing adjustments.
    #[builder(default)]
    #[serde(
        default,
        skip_serializing_if = "crate::instruments::ScenarioPricingOverrides::is_empty"
    )]
    pub scenario_pricing_overrides: crate::instruments::ScenarioPricingOverrides,

    /// Attributes for scenario selection.
    pub attributes: Attributes,

    /// Credit model configuration (prepayment/default/recovery + optional stochastic specs).
    ///
    /// Serialized keys are flattened for flat JSON layout.
    #[builder(default)]
    #[serde(default, flatten)]
    pub credit_model: CreditModelConfig,

    /// Market conditions impacting behavior.
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior.
    pub credit_factors: CreditFactors,

    /// Deal metadata (counterparties, identifiers).
    #[serde(default)]
    pub deal_metadata: Metadata,

    /// Behavioral assumption overrides.
    #[serde(default)]
    pub behavior_overrides: Overrides,

    /// Default behavioral assumptions for the deal.
    #[serde(default)]
    pub default_assumptions: DefaultAssumptions,

    /// Interest rate swaps used to hedge basis or interest rate risk.
    #[serde(default)]
    pub hedge_swaps: Vec<InterestRateSwap>,

    /// Senior transaction fees paid ahead of every note.
    ///
    /// `None` (the default) skips the fee tier. Use [`Self::with_standard_fees`]
    /// to apply the deal-type calibration from `types/constants.rs`.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees: Option<DealFees>,

    /// Overcollateralization / interest-coverage triggers evaluated each period.
    ///
    /// Each entry names a tranche and the OC and/or IC level that must be
    /// maintained for it. When a test fails, the cure amount is diverted from
    /// the divertible (subordinated-interest / residual) tier to redeem senior
    /// notes. Empty (the default) means no coverage tests run.
    ///
    /// # Examples
    ///
    /// ```
    /// use finstack_quant_valuations::instruments::fixed_income::structured_credit::waterfall::CoverageTrigger;
    ///
    /// // Class A must maintain 120% OC and 115% IC.
    /// let trigger = CoverageTrigger {
    ///     tranche_id: "CLASS_A".to_string(),
    ///     oc_trigger: Some(1.20),
    ///     ic_trigger: Some(1.15),
    /// };
    /// assert_eq!(trigger.oc_trigger, Some(1.20));
    /// ```
    ///
    /// Use the fully-qualified waterfall type here: the public re-export
    /// `structured_credit::CoverageTrigger` is a different per-tranche
    /// breach-state record that shares the name.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverage_triggers: Vec<waterfall::CoverageTrigger>,

    /// Clean-up call pool factor threshold (percentage of original balance).
    ///
    /// When the pool factor (current balance / original balance) drops below
    /// this threshold, the deal is optionally redeemed and all outstanding
    /// tranche balances are returned. Industry standard: typically 10%.
    ///
    /// Set to `None` to disable clean-up call (default).
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_call_pct: Option<f64>,

    /// Declarative waterfall rules (available-funds caps, etc.) layered onto the
    /// base waterfall by `resolve_waterfall`. `None` reproduces the base
    /// waterfall exactly.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waterfall_rules: Option<WaterfallRules>,

    /// Custom payment waterfall used verbatim for pricing.
    ///
    /// `None` (the default) synthesizes the canonical sequential template from
    /// the tranche structure ([`Waterfall::standard_sequential`]) plus any
    /// [`Self::fees`]. When set, this waterfall is authoritative:
    /// [`Self::create_waterfall`] returns it (with deal-level
    /// [`Self::coverage_triggers`] appended) and no template is synthesized.
    /// [`Self::waterfall_rules`] overlays (AFC, step-down, shifting interest,
    /// controlled accumulation) still apply — they rewrite tiers generically by
    /// `payment_type`, so they compose with custom structures.
    ///
    /// The waterfall also **defines each tranche's interest claim** (not just
    /// cash allocation): an uncapped `TrancheInterest` recipient owes the full
    /// coupon accrual, a `CappedTrancheInterest` recipient owes the capped
    /// coupon (the capped-off portion is never owed and never defers), and a
    /// debt tranche with **no** interest recipient owes nothing (a
    /// principal-only class). Equity is exempt: its interest/principal split
    /// is a reporting convention driven by the tranche's metadata coupon.
    ///
    /// Constraints, enforced by [`Self::with_waterfall`] and re-checked at
    /// pricing time so JSON-supplied deals get identical errors:
    /// - every tranche referenced by a tier recipient or coverage trigger must
    ///   exist in `tranches`, and tranche-keyed recipients must not target an
    ///   equity tranche (the engine records equity flows under
    ///   [`RecipientType::Equity`], paid via `ResidualCash`);
    /// - at most one interest-type recipient may name a given tranche (the
    ///   claim definition must be unambiguous);
    /// - `fees` must be `None` — senior fees are expressed as *leading*
    ///   [`PaymentType::Fee`] tiers of the custom waterfall, which then feed
    ///   the IC numerator and excess-spread/reserve sizing exactly like
    ///   template fees (fee tiers ranked below note interest are junior fees
    ///   and are deliberately not netted as senior claims);
    /// - the waterfall's `base_currency` must match the pool currency.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waterfall: Option<Waterfall>,
}
