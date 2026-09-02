//! Servicing and guarantee fee handling for agency MBS.
//!
//! Agency MBS have fees deducted from the gross coupon (WAC) before
//! passing through interest to investors:
//!
//! - **Servicing fee**: Compensation to the servicer for collecting payments
//! - **Guarantee fee (g-fee)**: Compensation to the agency for credit guarantee
//!
//! Net coupon = WAC - servicing_fee - guarantee_fee

use finstack_quant_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_quant_core::types::Bps;
use rust_decimal::Decimal;

/// Fee specification for MBS servicing or guarantee fees.
///
/// Represents a periodic fee based on the outstanding balance,
/// expressed in basis points.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub struct MbsFeeSpec {
    /// Fee name (e.g., "servicing", "guarantee")
    pub name: String,
    /// Annual fee rate in basis points (e.g., 25.0 for 25 bp)
    pub annual_rate_bp: f64,
    /// Day count convention for accrual
    pub day_count: DayCount,
    /// Payment frequency
    pub frequency: Tenor,
}

impl MbsFeeSpec {
    /// Create a servicing fee specification.
    ///
    /// # Arguments
    ///
    /// * `annual_rate_bp` - Annual servicing fee rate in basis points
    ///
    /// # Returns
    ///
    /// Fee spec with standard MBS conventions (30/360, monthly)
    pub fn servicing(annual_rate_bp: f64) -> Self {
        Self {
            name: "servicing".to_string(),
            annual_rate_bp,
            day_count: DayCount::Thirty360,
            frequency: Tenor::monthly(),
        }
    }

    /// Create a servicing fee specification using a typed basis-point rate.
    ///
    /// # Arguments
    ///
    /// * `annual_rate_bp` - Annual servicing fee in basis points, preserved
    ///   without float conversion in the standard monthly 30/360 convention.
    pub fn servicing_bp(annual_rate_bp: Bps) -> Self {
        Self {
            name: "servicing".to_string(),
            annual_rate_bp: annual_rate_bp.as_bp() as f64,
            day_count: DayCount::Thirty360,
            frequency: Tenor::monthly(),
        }
    }

    /// Create a guarantee fee (g-fee) specification.
    ///
    /// # Arguments
    ///
    /// * `annual_rate_bp` - Annual guarantee fee rate in basis points
    ///
    /// # Returns
    ///
    /// Fee spec with standard MBS conventions (30/360, monthly)
    pub fn guarantee(annual_rate_bp: f64) -> Self {
        Self {
            name: "guarantee_fee".to_string(),
            annual_rate_bp,
            day_count: DayCount::Thirty360,
            frequency: Tenor::monthly(),
        }
    }

    /// Create a guarantee fee specification using a typed basis-point rate.
    ///
    /// # Arguments
    ///
    /// * `annual_rate_bp` - Annual agency guarantee fee in basis points,
    ///   preserved in the standard monthly 30/360 convention.
    pub fn guarantee_bp(annual_rate_bp: Bps) -> Self {
        Self {
            name: "guarantee_fee".to_string(),
            annual_rate_bp: annual_rate_bp.as_bp() as f64,
            day_count: DayCount::Thirty360,
            frequency: Tenor::monthly(),
        }
    }

    /// Get the annual fee rate as a decimal.
    pub fn annual_rate(&self) -> f64 {
        self.annual_rate_bp / 10_000.0
    }

    /// Calculate the fee amount for a given balance and accrual period.
    ///
    /// # Arguments
    ///
    /// * `balance` - Outstanding balance
    /// * `accrual_days` - Number of days in the accrual period
    ///
    /// # Returns
    ///
    /// Fee amount for the period
    pub fn calculate_fee(&self, balance: f64, accrual_days: u32) -> f64 {
        let day_count_divisor = match self.day_count {
            DayCount::Act365F => 365.0,
            _ => 360.0,
        };

        balance * self.annual_rate() * (accrual_days as f64 / day_count_divisor)
    }
}

/// Convert servicing fee spec to generic FeeSpec for cashflow builder.
///
/// This allows integration with the existing fee emission infrastructure
/// in the cashflow builder.
///
/// # Arguments
///
/// * `mbs_fee` - Agency MBS servicing or guarantee-fee specification to map
///   into a periodic drawn-balance fee under its own frequency and day count.
pub fn to_cashflow_fee_spec(mbs_fee: &MbsFeeSpec) -> crate::cashflow::builder::FeeSpec {
    use crate::cashflow::builder::{FeeBase, FeeSpec};

    debug_assert!(
        mbs_fee.annual_rate_bp.is_finite(),
        "to_cashflow_fee_spec: annual_rate_bp is not finite ({})",
        mbs_fee.annual_rate_bp
    );
    FeeSpec::PeriodicBp {
        base: FeeBase::Drawn,
        // Convert f64 bp to Decimal for exact representation
        bp: Decimal::try_from(mbs_fee.annual_rate_bp).unwrap_or(Decimal::ZERO),
        frequency: mbs_fee.frequency,
        day_count: mbs_fee.day_count,
        business_day_convention: BusinessDayConvention::Following,
        calendar_id: "weekends_only".to_string(),
        stub: StubKind::None,
        accrual_basis: Default::default(),
    }
}

/// Calculate net coupon after fees.
///
/// # Arguments
///
/// * `gross_wac` - Weighted average coupon on underlying mortgages
/// * `servicing_rate` - Annual servicing fee rate (decimal)
/// * `guarantee_rate` - Annual guarantee fee rate (decimal)
///
/// # Returns
///
/// Net pass-through rate to investor
pub fn net_coupon(gross_wac: f64, servicing_rate: f64, guarantee_rate: f64) -> f64 {
    gross_wac - servicing_rate - guarantee_rate
}

/// Calculate gross WAC from net coupon and fees.
///
/// # Arguments
///
/// * `net_rate` - Pass-through rate to investor
/// * `servicing_rate` - Annual servicing fee rate (decimal)
/// * `guarantee_rate` - Annual guarantee fee rate (decimal)
///
/// # Returns
///
/// Gross weighted average coupon
pub fn gross_wac(net_rate: f64, servicing_rate: f64, guarantee_rate: f64) -> f64 {
    net_rate + servicing_rate + guarantee_rate
}

/// Standard agency fee rates by program.
///
/// These are typical ranges; actual fees vary by loan characteristics.
#[derive(Debug, Clone)]
pub struct AgencyFeeRates {
    /// Servicing fee in basis points
    pub servicing_bp: f64,
    /// Guarantee fee in basis points
    pub guarantee_bp: f64,
}

impl AgencyFeeRates {
    /// Typical FNMA fee rates (25 bp each).
    pub fn fnma_standard() -> Self {
        Self {
            servicing_bp: 25.0,
            guarantee_bp: 25.0,
        }
    }

    /// Typical GNMA fee rates (lower g-fee due to government backing).
    pub fn gnma_standard() -> Self {
        Self {
            servicing_bp: 25.0,
            guarantee_bp: 6.0, // Lower g-fee for government-backed
        }
    }

    /// Total fee strip in basis points.
    pub fn total_bp(&self) -> f64 {
        self.servicing_bp + self.guarantee_bp
    }

    /// Total fee strip as decimal rate.
    pub fn total_rate(&self) -> f64 {
        self.total_bp() / 10_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_servicing_fee_spec() {
        let fee = MbsFeeSpec::servicing(25.0);
        assert_eq!(fee.name, "servicing");
        assert!((fee.annual_rate() - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn test_guarantee_fee_spec() {
        let fee = MbsFeeSpec::guarantee(25.0);
        assert_eq!(fee.name, "guarantee_fee");
        assert!((fee.annual_rate() - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_fee() {
        let fee = MbsFeeSpec::servicing(25.0);
        let balance = 1_000_000.0;
        let accrual_days = 30;

        // 25 bp annual on 1M for 30 days (30/360)
        // = 1,000,000 * 0.0025 * (30/360) = 208.33
        let amount = fee.calculate_fee(balance, accrual_days);
        assert!((amount - 208.33).abs() < 0.01);
    }

    #[test]
    fn test_net_coupon_calculation() {
        let wac = 0.045; // 4.5%
        let servicing = 0.0025; // 25 bp
        let guarantee = 0.0025; // 25 bp

        let net = net_coupon(wac, servicing, guarantee);
        assert!((net - 0.04).abs() < 1e-10); // 4.0%
    }

    #[test]
    fn test_gross_wac_calculation() {
        let net = 0.04;
        let servicing = 0.0025;
        let guarantee = 0.0025;

        let gross = gross_wac(net, servicing, guarantee);
        assert!((gross - 0.045).abs() < 1e-10);
    }

    #[test]
    fn test_agency_fee_rates() {
        let fnma = AgencyFeeRates::fnma_standard();
        assert_eq!(fnma.total_bp(), 50.0);
        assert!((fnma.total_rate() - 0.005).abs() < 1e-10);

        let gnma = AgencyFeeRates::gnma_standard();
        assert!(gnma.guarantee_bp < fnma.guarantee_bp);
    }
}
