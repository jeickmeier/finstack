//! Swaption payoffs for Monte Carlo pricing.
//!
//! Implements Bermudan swaption pricing using Longstaff-Schwartz Monte Carlo.
//! A swaption is an option to enter into an interest rate swap at future dates.

use crate::instruments::common_impl::parameters::OptionType;

/// Swap schedule for Monte Carlo pricing.
///
/// Stores payment dates and accrual fractions for computing swap rates
/// and annuities from Hull-White short rate simulations.
#[derive(Debug, Clone)]
pub struct SwapSchedule {
    /// Payment dates (time in years from valuation date)
    pub payment_dates: Vec<f64>,
    /// Accrual fractions (daycount) for each period
    pub accrual_fractions: Vec<f64>,
    /// Start date of swap (time in years)
    pub start_date: f64,
    /// End date of swap (time in years)
    pub end_date: f64,
}

impl SwapSchedule {
    /// Create a new swap schedule.
    ///
    /// # Arguments
    ///
    /// * `start_date` - Swap start date (time in years)
    /// * `end_date` - Swap end date (time in years)
    /// * `payment_dates` - Payment dates (must be sorted, within [start_date, end_date])
    /// * `accrual_fractions` - Accrual fractions for each period
    ///
    /// # Errors
    ///
    /// Returns [`finstack_quant_core::Error::Validation`] if `payment_dates` and
    /// `accrual_fractions` differ in length, `start_date >= end_date`, the
    /// payment dates are not strictly sorted ascending, or any payment date
    /// falls outside `[start_date, end_date]`.
    pub fn new(
        start_date: f64,
        end_date: f64,
        payment_dates: Vec<f64>,
        accrual_fractions: Vec<f64>,
    ) -> finstack_quant_core::Result<Self> {
        use std::cmp::Ordering;

        if payment_dates.len() != accrual_fractions.len() {
            return Err(finstack_quant_core::Error::Validation(format!(
                "SwapSchedule: payment_dates ({}) and accrual_fractions ({}) must have the same length",
                payment_dates.len(),
                accrual_fractions.len()
            )));
        }
        if start_date.partial_cmp(&end_date) != Some(Ordering::Less) {
            return Err(finstack_quant_core::Error::Validation(format!(
                "SwapSchedule: start_date ({start_date}) must be strictly before end_date ({end_date})"
            )));
        }
        // Verify payment dates are strictly sorted ascending and within range.
        for (i, &date) in payment_dates.iter().enumerate() {
            if i > 0 && payment_dates[i - 1].partial_cmp(&date) != Some(Ordering::Less) {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "SwapSchedule: payment_dates must be strictly sorted ascending; \
                     found {} >= {} at index {}",
                    payment_dates[i - 1],
                    date,
                    i
                )));
            }
            if date.is_nan() || date < start_date || date > end_date {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "SwapSchedule: payment date {date} at index {i} is outside \
                     [start_date {start_date}, end_date {end_date}]"
                )));
            }
        }

        Ok(Self {
            payment_dates,
            accrual_fractions,
            start_date,
            end_date,
        })
    }

    /// Compute annuity (PV01) at time t from discount factors.
    ///
    /// A(t) = Σ τ_i * DF(t, T_i) where τ_i are accrual fractions.
    #[cfg(test)]
    fn annuity(&self, discount_factors: &[f64]) -> f64 {
        assert_eq!(
            discount_factors.len(),
            self.payment_dates.len(),
            "Discount factors must match payment dates"
        );

        self.accrual_fractions
            .iter()
            .zip(discount_factors.iter())
            .map(|(tau, df)| tau * df)
            .sum()
    }
}

/// Bermudan swaption payoff.
///
/// A Bermudan swaption allows exercise at multiple dates before maturity.
/// At each exercise date, the holder can choose to enter into a swap with
/// fixed rate equal to the strike.
///
/// # Payoff
///
/// At exercise date t, if exercised:
/// - Payer: Pay fixed rate K, receive floating → value = (S(t) - K)⁺ · A(t) · N
/// - Receiver: Receive fixed rate K, pay floating → value = (K - S(t))⁺ · A(t) · N
///
/// where `S(t)` is the forward swap rate, `A(t)` is the **swap annuity** at the
/// exercise date and `N` the notional. The exercise dates are supplied to the
/// engine as time-grid step indices; the Longstaff-Schwartz induction in
/// `monte_carlo_lsmc.rs` annuitises and discounts the exercise value directly.
#[derive(Debug, Clone)]
pub struct BermudanSwaptionPayoff {
    /// Swap schedule
    pub swap_schedule: SwapSchedule,
    /// Strike rate (fixed rate of the swap)
    pub strike: f64,
    /// [`OptionType::Call`] is a payer swaption (right to pay fixed),
    /// [`OptionType::Put`] a receiver swaption (right to receive fixed).
    pub option_type: OptionType,
    /// Notional amount
    pub notional: f64,
}

impl BermudanSwaptionPayoff {
    /// Create a new Bermudan swaption payoff.
    ///
    /// # Arguments
    ///
    /// * `swap_schedule` - Underlying swap schedule
    /// * `strike` - Fixed rate of the swap (e.g., 0.0325 for 3.25%)
    /// * `option_type` - `Call` for a payer swaption, `Put` for a receiver
    /// * `notional` - Notional amount
    pub fn new(
        swap_schedule: SwapSchedule,
        strike: f64,
        option_type: OptionType,
        notional: f64,
    ) -> Self {
        Self {
            swap_schedule,
            strike,
            option_type,
            notional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_schedule_creation() {
        let payment_dates = vec![1.0, 1.25, 1.5, 1.75, 2.0];
        let accruals = vec![0.25, 0.25, 0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 2.0, payment_dates, accruals)
            .expect("valid swap schedule inputs");

        assert_eq!(schedule.start_date, 1.0);
        assert_eq!(schedule.end_date, 2.0);
        assert_eq!(schedule.payment_dates.len(), 5);
    }

    #[test]
    fn test_swap_schedule_annuity() {
        let payment_dates = vec![1.0, 1.25, 1.5];
        let accruals = vec![0.25, 0.25, 0.25];
        let schedule = SwapSchedule::new(1.0, 1.5, payment_dates, accruals)
            .expect("valid swap schedule inputs");

        let discount_factors = vec![0.95, 0.94, 0.93];
        let annuity = schedule.annuity(&discount_factors);

        // Annuity = 0.25 * 0.95 + 0.25 * 0.94 + 0.25 * 0.93 = 0.705
        assert!((annuity - 0.705).abs() < 1e-10);
    }

    #[test]
    fn test_swap_schedule_rejects_degenerate_inputs() {
        // Mismatched lengths between payment dates and accrual fractions.
        assert!(SwapSchedule::new(1.0, 2.0, vec![1.0, 1.5], vec![0.25]).is_err());

        // start_date not before end_date.
        assert!(SwapSchedule::new(2.0, 1.0, vec![], vec![]).is_err());

        // Unsorted payment dates.
        assert!(SwapSchedule::new(1.0, 2.0, vec![1.5, 1.0], vec![0.25, 0.25]).is_err());

        // Payment date outside [start_date, end_date].
        assert!(SwapSchedule::new(1.0, 2.0, vec![2.5], vec![0.25]).is_err());
    }
}
