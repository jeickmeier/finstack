//! Dedicated regression target for the shared valuation test helpers.
//!
//! Keeping these tests out of `tests/common` prevents every integration target
//! that imports the helpers from compiling and executing the same checks.

#![allow(dead_code, unused_imports)]

#[path = "common/mod.rs"]
mod common;

mod assertions_tests {
    use crate::common::{assertions::*, tolerances};

    #[test]
    fn test_assert_approx_eq_passes() {
        assert_approx_eq(1.0, 1.0 + 1e-11, tolerances::TIGHT);
    }

    #[test]
    fn test_assert_approx_eq_exact_match() {
        assert_approx_eq(42.0, 42.0, tolerances::TIGHT);
    }

    #[test]
    fn test_assert_approx_eq_at_tolerance_boundary() {
        assert_approx_eq(1.0, 1.0 + tolerances::STANDARD, tolerances::STANDARD);
    }

    #[test]
    #[should_panic(expected = "values not approximately equal")]
    fn test_assert_approx_eq_fails() {
        assert_approx_eq(1.0, 2.0, tolerances::TIGHT);
    }

    #[test]
    fn test_assert_approx_eq_std_passes() {
        assert_approx_eq_std(1.0, 1.0 + 1e-7);
    }

    #[test]
    #[should_panic(expected = "values not approximately equal")]
    fn test_assert_approx_eq_std_fails_beyond_standard_tolerance() {
        assert_approx_eq_std(1.0, 1.0 + 1e-5);
    }

    #[test]
    fn test_assert_relative_eq_passes() {
        assert_relative_eq(100.5, 100.0, tolerances::PERCENT_1);
    }

    #[test]
    fn test_assert_relative_eq_near_zero_falls_back_to_absolute() {
        assert_relative_eq(5e-11, 0.0, tolerances::PERCENT_1);
    }

    #[test]
    #[should_panic(expected = "relative difference too large")]
    fn test_assert_relative_eq_fails() {
        assert_relative_eq(110.0, 100.0, tolerances::PERCENT_1);
    }

    #[test]
    #[should_panic(expected = "values not approximately equal")]
    fn test_assert_relative_eq_near_zero_fails_when_outside_scaled_abs_tol() {
        assert_relative_eq(2e-10, 0.0, tolerances::PERCENT_1);
    }

    #[test]
    fn test_assert_positive_passes() {
        assert_positive(0.001, "small positive");
        assert_positive(1_000_000.0, "large positive");
        assert_positive(f64::MIN_POSITIVE, "min positive");
    }

    #[test]
    #[should_panic(expected = "should be positive")]
    fn test_assert_positive_fails_for_zero() {
        assert_positive(0.0, "zero");
    }

    #[test]
    #[should_panic(expected = "should be positive")]
    fn test_assert_positive_fails_for_negative() {
        assert_positive(-1.0, "negative");
    }

    #[test]
    fn test_assert_negative_passes() {
        assert_negative(-0.001, "small negative");
        assert_negative(-1_000_000.0, "large negative");
        assert_negative(f64::MIN, "min f64");
    }

    #[test]
    #[should_panic(expected = "should be negative")]
    fn test_assert_negative_fails_for_zero() {
        assert_negative(0.0, "zero");
    }

    #[test]
    #[should_panic(expected = "should be negative")]
    fn test_assert_negative_fails_for_positive() {
        assert_negative(1.0, "positive");
    }

    #[test]
    fn test_assert_non_negative_passes_for_positive() {
        assert_non_negative(1.0, "positive");
    }

    #[test]
    fn test_assert_non_negative_passes_for_zero() {
        assert_non_negative(0.0, "zero");
    }

    #[test]
    #[should_panic(expected = "should be non-negative")]
    fn test_assert_non_negative_fails_for_negative() {
        assert_non_negative(-0.001, "small negative");
    }

    #[test]
    fn test_assert_finite_passes_for_normal_values() {
        assert_finite(0.0, "zero");
        assert_finite(1.0, "one");
        assert_finite(-1.0, "negative one");
        assert_finite(f64::MAX, "max f64");
        assert_finite(f64::MIN, "min f64");
        assert_finite(f64::MIN_POSITIVE, "min positive");
    }

    #[test]
    #[should_panic(expected = "should be finite")]
    fn test_assert_finite_fails_for_nan() {
        assert_finite(f64::NAN, "NaN");
    }

    #[test]
    #[should_panic(expected = "should be finite")]
    fn test_assert_finite_fails_for_positive_infinity() {
        assert_finite(f64::INFINITY, "positive infinity");
    }

    #[test]
    #[should_panic(expected = "should be finite")]
    fn test_assert_finite_fails_for_negative_infinity() {
        assert_finite(f64::NEG_INFINITY, "negative infinity");
    }

    #[test]
    fn test_assert_in_range_passes_for_value_in_range() {
        assert_in_range(5.0, 0.0, 10.0, "mid range");
    }

    #[test]
    fn test_assert_in_range_passes_at_lower_bound() {
        assert_in_range(0.0, 0.0, 10.0, "at lower bound");
    }

    #[test]
    fn test_assert_in_range_passes_at_upper_bound() {
        assert_in_range(10.0, 0.0, 10.0, "at upper bound");
    }

    #[test]
    #[should_panic(expected = "should be in range")]
    fn test_assert_in_range_fails_below_range() {
        assert_in_range(-0.001, 0.0, 10.0, "below range");
    }

    #[test]
    #[should_panic(expected = "should be in range")]
    fn test_assert_in_range_fails_above_range() {
        assert_in_range(10.001, 0.0, 10.0, "above range");
    }

    #[test]
    fn test_assert_in_range_negative_bounds() {
        assert_in_range(-5.0, -10.0, 0.0, "negative range");
    }
}

mod builders_tests {
    use crate::common::{assertions::assert_approx_eq, builders::*, tolerances};
    use finstack_quant_core::market_data::scalars::MarketScalar;
    use finstack_quant_core::types::CurveId;
    use finstack_quant_valuations::instruments::OptionType;
    use time::macros::date;

    #[test]
    fn test_market_builder_defaults() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).build();

        assert!(market.get_discount("USD-OIS").is_ok());
        assert!(market.get_surface("SPOT_VOL").is_ok());
        assert!(market.get_price("SPOT").is_ok());
    }

    #[test]
    fn test_market_builder_custom_spot() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).spot(150.0).build();

        let spot = market.get_price("SPOT").expect("spot should exist");
        let spot_value = match spot {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        assert_approx_eq(spot_value, 150.0, tolerances::TIGHT);
    }

    #[test]
    fn test_market_builder_custom_vol() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).vol(0.30).build();

        let surface = market
            .get_surface("SPOT_VOL")
            .expect("vol surface should exist");
        let vol = surface
            .value_checked(1.0, 100.0)
            .expect("vol lookup should succeed");
        assert_approx_eq(vol, 0.30, tolerances::TIGHT);
    }

    #[test]
    fn test_market_builder_custom_rate() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).rate(0.03).build();

        let curve = market.get_discount("USD-OIS").expect("curve should exist");
        let expected_df = (-0.03_f64).exp();
        assert_approx_eq(curve.df(1.0), expected_df, tolerances::STANDARD);
    }

    #[test]
    fn test_market_builder_with_dividend_yield() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).div_yield(0.02).build();
        assert!(market.get_price("SPOT_DIV").is_ok());
    }

    #[test]
    fn test_market_builder_custom_ids() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of)
            .discount_curve_id("CUSTOM-DISC")
            .vol_surface_id("CUSTOM-VOL")
            .spot_id("CUSTOM-SPOT")
            .build();

        assert!(market.get_discount("CUSTOM-DISC").is_ok());
        assert!(market.get_surface("CUSTOM-VOL").is_ok());
        assert!(market.get_price("CUSTOM-SPOT").is_ok());
    }

    #[test]
    fn test_market_builder_updates_div_yield_id_when_spot_id_changes() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of)
            .div_yield(0.02)
            .spot_id("CUSTOM-SPOT")
            .build();

        assert!(market.get_price("CUSTOM-SPOT_DIV").is_ok());
        assert!(market.get_price("SPOT_DIV").is_err());
    }

    #[test]
    fn test_market_builder_custom_tenor() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of)
            .tenor_years(5.0)
            .rate(0.04)
            .build();

        let curve = market.get_discount("USD-OIS").expect("curve should exist");
        let df_5y = curve.df(5.0);
        assert!(df_5y > 0.0 && df_5y < 1.0);
    }

    #[test]
    fn test_option_builder_defaults() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry).build();

        assert_eq!(option.strike, 100.0);
        assert!(matches!(option.option_type, OptionType::Call));
        assert_eq!(option.notional.amount(), 100.0);
        assert_eq!(option.expiry, expiry);
    }

    #[test]
    fn test_option_builder_custom_strike() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry).strike(120.0).build();
        assert_eq!(option.strike, 120.0);
    }

    #[test]
    fn test_option_builder_put_option() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry)
            .option_type(OptionType::Put)
            .build();
        assert!(matches!(option.option_type, OptionType::Put));
    }

    #[test]
    fn test_option_builder_custom_contract_size() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry).contract_size(50.0).build();
        assert_eq!(option.notional.amount(), 50.0);
    }

    #[test]
    fn test_option_builder_custom_ids() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry)
            .id("CUSTOM-OPT")
            .discount_curve_id("CUSTOM-DISC")
            .spot_id("CUSTOM-SPOT")
            .vol_surface_id("CUSTOM-VOL")
            .build();

        assert_eq!(option.id.as_str(), "CUSTOM-OPT");
        assert_eq!(option.discount_curve_id.as_str(), "CUSTOM-DISC");
        assert_eq!(option.spot_id.as_str(), "CUSTOM-SPOT");
        assert_eq!(option.vol_surface_id.as_str(), "CUSTOM-VOL");
    }

    #[test]
    fn test_option_builder_with_dividend_yield() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry)
            .div_yield_id("DIV-YIELD")
            .build();
        assert_eq!(option.div_yield_id, Some(CurveId::new("DIV-YIELD")));
    }

    #[test]
    fn test_simple_option_market() {
        let as_of = date!(2024 - 01 - 01);
        let market = simple_option_market(as_of, 100.0, 0.20, 0.05);

        assert!(market.get_discount("USD-OIS").is_ok());
        assert!(market.get_surface("SPOT_VOL").is_ok());
        assert!(market.get_price("SPOT").is_ok());

        let spot = market.get_price("SPOT").expect("spot should exist");
        let spot_value = match spot {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        assert_approx_eq(spot_value, 100.0, tolerances::TIGHT);
    }

    #[test]
    fn test_simple_option_market_with_different_params() {
        let as_of = date!(2024 - 06 - 15);
        let market = simple_option_market(as_of, 150.0, 0.35, 0.02);

        let spot = market.get_price("SPOT").expect("spot should exist");
        let spot_value = match spot {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        assert_approx_eq(spot_value, 150.0, tolerances::TIGHT);

        let curve = market.get_discount("USD-OIS").expect("curve should exist");
        let expected_df = (-0.02_f64).exp();
        assert_approx_eq(curve.df(1.0), expected_df, tolerances::STANDARD);
    }

    #[test]
    fn test_option_market_with_divs() {
        let as_of = date!(2024 - 01 - 01);
        let market = option_market_with_divs(as_of, 100.0, 0.25, 0.05, 0.02);

        assert!(market.get_price("SPOT_DIV").is_ok());
        let div = market
            .get_price("SPOT_DIV")
            .expect("dividend yield should exist");
        let div_value = match div {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        assert_approx_eq(div_value, 0.02, tolerances::TIGHT);
    }

    #[test]
    fn test_simple_call_option() {
        let expiry = date!(2025 - 06 - 30);
        let option = simple_call_option(expiry, 110.0);
        assert!(matches!(option.option_type, OptionType::Call));
        assert_eq!(option.strike, 110.0);
        assert_eq!(option.expiry, expiry);
    }

    #[test]
    fn test_simple_put_option() {
        let expiry = date!(2025 - 06 - 30);
        let option = simple_put_option(expiry, 90.0);
        assert!(matches!(option.option_type, OptionType::Put));
        assert_eq!(option.strike, 90.0);
        assert_eq!(option.expiry, expiry);
    }

    #[test]
    fn test_test_option_returns_builder() {
        let expiry = date!(2025 - 01 - 01);
        let option = test_option(expiry).strike(120.0).build();
        assert_eq!(option.strike, 120.0);
    }

    #[test]
    fn test_test_market_returns_builder() {
        let as_of = date!(2024 - 01 - 01);
        let market = test_market(as_of).spot(200.0).vol(0.40).build();

        let spot = market.get_price("SPOT").expect("spot should exist");
        let spot_value = match spot {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => m.amount(),
        };
        assert_approx_eq(spot_value, 200.0, tolerances::TIGHT);
    }

    #[test]
    fn test_market_builder_zero_rate() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).rate(0.0).build();
        let curve = market.get_discount("USD-OIS").expect("curve should exist");
        assert_approx_eq(curve.df(1.0), 1.0, tolerances::TIGHT);
    }

    #[test]
    fn test_market_builder_low_vol() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).vol(0.01).build();
        assert!(market.get_surface("SPOT_VOL").is_ok());
    }

    #[test]
    fn test_market_builder_high_vol() {
        let as_of = date!(2024 - 01 - 01);
        let market = TestMarketBuilder::new(as_of).vol(1.0).build();
        assert!(market.get_surface("SPOT_VOL").is_ok());
    }

    #[test]
    fn test_option_builder_deep_itm_strike() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry).strike(10.0).build();
        assert_eq!(option.strike, 10.0);
    }

    #[test]
    fn test_option_builder_deep_otm_strike() {
        let expiry = date!(2025 - 01 - 01);
        let option = TestOptionBuilder::new(expiry).strike(500.0).build();
        assert_eq!(option.strike, 500.0);
    }
}

mod fixtures_tests {
    use crate::common::{assertions::assert_approx_eq, fixtures::*, tolerances};
    use finstack_quant_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_base_date_is_valid_business_day() {
        let date = base_date();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 2);
        assert!(
            date.weekday() != time::Weekday::Saturday && date.weekday() != time::Weekday::Sunday,
            "base_date should be a weekday"
        );
    }

    #[test]
    fn test_imm_base_date_is_march_20() {
        let date = imm_base_date();
        assert_eq!(date.year(), 2025);
        assert_eq!(date.month(), Month::March);
        assert_eq!(date.day(), 20);
    }

    #[test]
    fn test_usd_discount_curve_construction() {
        let date = base_date();
        let curve = usd_discount_curve(date, "USD-OIS");
        assert_eq!(curve.id().as_str(), "USD-OIS");
        assert_eq!(curve.base_date(), date);
    }

    #[test]
    fn test_usd_discount_curve_df_at_zero_is_one() {
        let curve = usd_discount_curve(base_date(), "TEST");
        assert_approx_eq(curve.df(0.0), 1.0, TIGHT);
    }

    #[test]
    fn test_usd_discount_curve_df_values_match_provenance() {
        let curve = usd_discount_curve(base_date(), "TEST");
        assert_approx_eq(curve.df(1.0), 0.98, STANDARD);
        assert_approx_eq(curve.df(3.0), 0.94, STANDARD);
        assert_approx_eq(curve.df(5.0), 0.90, STANDARD);
        assert_approx_eq(curve.df(10.0), 0.80, STANDARD);
    }

    #[test]
    fn test_usd_discount_curve_implied_rates_are_reasonable() {
        let curve = usd_discount_curve(base_date(), "TEST");
        for t in [1.0, 3.0, 5.0, 10.0] {
            let df = curve.df(t);
            let implied_rate = -df.ln() / t;
            assert!(
                (0.01..=0.05).contains(&implied_rate),
                "Implied rate at {t}Y should be in [1%, 5%], got {:.2}%",
                implied_rate * 100.0
            );
        }
    }

    #[test]
    fn test_usd_discount_curve_is_monotonically_decreasing() {
        let curve = usd_discount_curve(base_date(), "TEST");
        let tenors = [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0];
        for i in 1..tenors.len() {
            let df_prev = curve.df(tenors[i - 1]);
            let df_curr = curve.df(tenors[i]);
            assert!(
                df_curr < df_prev,
                "DF should decrease: DF({}) = {} >= DF({}) = {}",
                tenors[i],
                df_curr,
                tenors[i - 1],
                df_prev
            );
        }
    }

    #[test]
    fn test_usd_discount_curve_minimal_construction() {
        let date = base_date();
        let curve = usd_discount_curve_minimal(date, "USD-OIS-MIN");
        assert_eq!(curve.id().as_str(), "USD-OIS-MIN");
        assert_approx_eq(curve.df(0.0), 1.0, TIGHT);
        assert_approx_eq(curve.df(1.0), 0.98, STANDARD);
        assert_approx_eq(curve.df(5.0), 0.90, STANDARD);
    }

    #[test]
    fn test_usd_discount_curve_monotone_convex_construction() {
        let date = base_date();
        let curve = usd_discount_curve_monotone_convex(date, "USD-MC");
        assert_eq!(curve.id().as_str(), "USD-MC");
        assert_approx_eq(curve.df(0.0), 1.0, TIGHT);
        assert_approx_eq(curve.df(0.25), 0.9888, STANDARD);
        assert_approx_eq(curve.df(1.0), 0.9550, STANDARD);
    }

    #[test]
    fn test_market_context_with_usd_discount() {
        let market = market_context_with_usd_discount(base_date());
        assert!(market.get_discount("USD-OIS").is_ok());
    }

    #[test]
    fn test_market_context_with_minimal_discount() {
        let market = market_context_with_minimal_discount(base_date());
        let curve = market.get_discount("USD-OIS").expect("curve should exist");
        assert_approx_eq(curve.df(1.0), 0.98, STANDARD);
    }

    #[test]
    fn test_standard_notional_value() {
        assert_eq!(STANDARD_NOTIONAL, 1_000_000.0);
    }

    #[test]
    fn test_usd_currency_constant() {
        assert_eq!(USD, Currency::USD);
    }

    #[test]
    fn test_implied_test_rate_matches_curve() {
        let curve = usd_discount_curve(base_date(), "TEST");
        let actual_rate = -curve.df(1.0).ln();
        assert_approx_eq(actual_rate, IMPLIED_TEST_RATE, tolerances::LOOSE);
    }
}

mod tolerances_tests {
    use crate::common::tolerances::*;
    use std::hint::black_box;

    #[test]
    fn test_tolerance_hierarchy() {
        assert!(black_box(TIGHT) < black_box(STANDARD));
        assert!(black_box(STANDARD) < black_box(LOOSE));
    }

    #[test]
    fn test_relative_tolerance_hierarchy() {
        assert!(black_box(PERCENT_001) < black_box(PERCENT_01));
        assert!(black_box(PERCENT_01) < black_box(PERCENT_1));
        assert!(black_box(PERCENT_1) < black_box(PERCENT_5));
    }

    #[test]
    fn test_near_zero_is_smaller_than_standard() {
        assert!(black_box(NEAR_ZERO) < black_box(STANDARD));
    }

    #[test]
    fn test_tolerance_values() {
        assert_eq!(TIGHT, 1e-10);
        assert_eq!(STANDARD, 1e-6);
        assert_eq!(LOOSE, 1e-3);
        assert_eq!(NEAR_ZERO, 1e-8);
    }

    #[test]
    fn test_percentage_values() {
        assert_eq!(PERCENT_001, 0.0001);
        assert_eq!(PERCENT_01, 0.001);
        assert_eq!(PERCENT_1, 0.01);
        assert_eq!(PERCENT_5, 0.05);
    }
}
