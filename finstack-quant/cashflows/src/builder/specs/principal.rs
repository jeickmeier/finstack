//! Principal-exchange policy for [`CashFlowBuilder`](crate::builder::CashFlowBuilder).

/// Whether the builder emits issue-funding and maturity-redemption notionals.
///
/// Outstanding still starts at the configured initial principal for coupon
/// math. Scheduled [`AmortizationSpec`](super::AmortizationSpec) payments and
/// explicit principal events still emit. This is not the cross-currency
/// `NotionalExchange` policy (no final-only or MTM-resetting variants).
///
/// # Variants
///
/// - **`None`**: track outstanding only; no `CFKind::Notional` issue or
///   redemption flows. Vanilla IRS and basis swaps use this.
/// - **`InitialAndFinal`** (default): emit issue funding and the maturity
///   balloon on the lagged redemption date.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_cashflows::builder::PrincipalExchange;
///
/// assert_eq!(
///     PrincipalExchange::default(),
///     PrincipalExchange::InitialAndFinal
/// );
/// ```
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalExchange {
    /// Do not emit issue or redemption `CFKind::Notional` flows.
    None,
    /// Emit issue funding and maturity redemption (default).
    #[default]
    InitialAndFinal,
}

impl PrincipalExchange {
    /// Returns `true` for the default [`Self::InitialAndFinal`] variant.
    #[must_use]
    pub fn is_initial_and_final(&self) -> bool {
        matches!(self, Self::InitialAndFinal)
    }
}
