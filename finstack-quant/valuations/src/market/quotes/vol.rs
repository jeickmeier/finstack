//! Volatility quote types for surface calibration.
//!
//! Defines the volatility quote types used for surface calibration of options and swaptions.
//! Volatility quotes include strike, expiry, and implied volatility values for building
//! volatility surfaces.

use super::ids::QuoteId;
use crate::instruments::OptionType;
use crate::market::conventions::ids::{
    CapFloorConventionId, OptionConventionId, SwaptionConventionId,
};
use finstack_quant_core::dates::Date;
use finstack_quant_core::market_data::surfaces::VolQuoteType;
use finstack_quant_core::types::UnderlyingId;
use finstack_quant_core::{Error, Result};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Parse a vol quote's `quote_type` string into a typed [`VolQuoteType`].
///
/// Accepts normal/Bachelier and lognormal/Black aliases case-insensitively.
///
/// # Arguments
///
/// * `quote_type` - Case-insensitive quote convention label.
///
/// # Errors
///
/// Returns [`Error::Validation`] for an unrecognized label.
///
/// # Examples
/// ```rust
/// use finstack_quant_valuations::market::quotes::vol::parse_vol_quote_type;
/// use finstack_quant_core::market_data::surfaces::VolQuoteType;
///
/// assert_eq!(parse_vol_quote_type("Normal").unwrap(), VolQuoteType::Normal);
/// assert_eq!(
///     parse_vol_quote_type("black_lognormal").unwrap(),
///     VolQuoteType::BlackLognormal
/// );
/// assert!(parse_vol_quote_type("implied_vol").is_err());
/// ```
pub fn parse_vol_quote_type(quote_type: &str) -> Result<VolQuoteType> {
    match quote_type.to_ascii_lowercase().as_str() {
        "normal" | "bachelier" => Ok(VolQuoteType::Normal),
        "lognormal" | "log_normal" | "black" | "black_lognormal" => {
            Ok(VolQuoteType::BlackLognormal)
        }
        other => Err(Error::Validation(format!(
            "Unrecognized vol quote_type '{other}': expected one of \
             'normal', 'bachelier', 'lognormal', 'log_normal', 'black', 'black_lognormal'"
        ))),
    }
}

/// Volatility quotes for option and swaption surface calibration.
///
/// Supports two types of volatility quotes:
/// 1. **Option volatility**: For equity, commodity, or FX options with strike and expiry
/// 2. **Swaption volatility**: For interest rate swaptions with strike, expiry, and underlying swap maturity date
///
/// # Examples
///
/// Option volatility quote:
/// ```rust
/// use finstack_quant_valuations::market::quotes::vol::VolQuote;
/// use finstack_quant_valuations::market::quotes::ids::QuoteId;
/// use finstack_quant_valuations::market::conventions::ids::OptionConventionId;
/// use finstack_quant_valuations::instruments::OptionType;
/// use finstack_quant_core::dates::Date;
/// use finstack_quant_core::types::UnderlyingId;
///
/// let quote = VolQuote::OptionVol {
///     id: QuoteId::new("SPX-VOL-20241220-4500"),
///     underlying: UnderlyingId::new("SPX"),
///     expiry: Date::from_calendar_date(2024, time::Month::December, 20).unwrap(),
///     strike: 4500.0,
///     vol: 0.20, // 20% implied volatility
///     option_type: OptionType::Call,
///     convention: OptionConventionId::new("USD-EQUITY"),
/// };
/// ```
///
/// Swaption volatility quote:
/// ```rust
/// use finstack_quant_valuations::market::quotes::vol::VolQuote;
/// use finstack_quant_valuations::market::quotes::ids::QuoteId;
/// use finstack_quant_valuations::market::conventions::ids::SwaptionConventionId;
/// use finstack_quant_core::dates::Date;
///
/// let quote = VolQuote::SwaptionVol {
///     id: QuoteId::new("USD-SWPTN-VOL-1Yx5Y-ATM"),
///     expiry: Date::from_calendar_date(2025, time::Month::June, 20).unwrap(),
///     maturity: Date::from_calendar_date(2030, time::Month::June, 20).unwrap(),
///     strike: 0.045, // 4.5% strike rate
///     vol: 0.15, // 15% implied volatility
///     quote_type: "Normal".to_string(),
///     convention: SwaptionConventionId::new("USD"),
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
// Keep this enum externally tagged. Market quote schemas, golden calibration
// payloads, and Python envelope payloads already depend on this shape.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum VolQuote {
    /// Equity or commodity option implied volatility quote.
    OptionVol {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Underlying identifier
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        underlying: UnderlyingId,
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        #[schemars(with = "String")]
        expiry: Date,
        /// Strike
        strike: f64,
        /// Implied volatility
        vol: f64,
        /// Option type (Call or Put).
        option_type: OptionType,
        /// Per-instrument conventions
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: OptionConventionId,
    },
    /// Interest rate swaption implied volatility quote.
    SwaptionVol {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        #[schemars(with = "String")]
        expiry: Date,
        /// Underlying swap maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        #[schemars(with = "String")]
        maturity: Date,
        /// Strike rate
        strike: f64,
        /// Implied volatility
        vol: f64,
        /// Quote type label; see [`parse_vol_quote_type`].
        quote_type: String,
        /// Option exercise conventions
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: SwaptionConventionId,
    },
    /// Interest rate cap/floor implied volatility quote.
    CapFloorVol {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Cap/floor maturity or caplet expiry.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        #[schemars(with = "String")]
        expiry: Date,
        /// Strike rate.
        strike: f64,
        /// Implied volatility.
        vol: f64,
        /// Quote type label, e.g. `"normal"`; see [`parse_vol_quote_type`].
        quote_type: String,
        /// `true` for cap, `false` for floor.
        is_cap: bool,
        /// Cap/floor market conventions.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: CapFloorConventionId,
    },
}

impl VolQuote {
    /// Get the unique identifier of the quote.
    pub fn id(&self) -> &QuoteId {
        match self {
            VolQuote::OptionVol { id, .. }
            | VolQuote::SwaptionVol { id, .. }
            | VolQuote::CapFloorVol { id, .. } => id,
        }
    }

    /// Create a new quote with the volatility bumped by an absolute amount.
    ///
    /// # Arguments
    ///
    /// * `vol_bump` - The bump amount in volatility terms (e.g., `0.01` for a +1 vol point bump)
    ///
    /// # Returns
    ///
    /// A new `VolQuote` with the bumped volatility.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_valuations::market::quotes::vol::VolQuote;
    /// use finstack_quant_valuations::market::quotes::ids::QuoteId;
    /// use finstack_quant_valuations::market::conventions::ids::OptionConventionId;
    /// use finstack_quant_valuations::instruments::OptionType;
    /// use finstack_quant_core::dates::Date;
    /// use finstack_quant_core::types::UnderlyingId;
    ///
    /// let quote = VolQuote::OptionVol {
    ///     id: QuoteId::new("SPX-VOL-20241220-4500"),
    ///     underlying: UnderlyingId::new("SPX"),
    ///     expiry: Date::from_calendar_date(2024, time::Month::December, 20).unwrap(),
    ///     strike: 4500.0,
    ///     vol: 0.20,
    ///     option_type: OptionType::Call,
    ///     convention: OptionConventionId::new("USD-EQUITY"),
    /// };
    ///
    /// // Bump by 1 vol point
    /// let bumped = quote.bump_vol_absolute(0.01);
    /// ```
    pub fn bump_vol_absolute(&self, vol_bump: f64) -> Self {
        match self {
            VolQuote::OptionVol {
                id,
                underlying,
                expiry,
                strike,
                vol,
                option_type,
                convention,
            } => VolQuote::OptionVol {
                id: id.clone(),
                underlying: underlying.clone(),
                expiry: *expiry,
                strike: *strike,
                vol: vol + vol_bump,
                option_type: *option_type,
                convention: convention.clone(),
            },
            VolQuote::SwaptionVol {
                id,
                expiry,
                maturity,
                strike,
                vol,
                quote_type,
                convention,
            } => VolQuote::SwaptionVol {
                id: id.clone(),
                expiry: *expiry,
                maturity: *maturity,
                strike: *strike,
                vol: vol + vol_bump,
                quote_type: quote_type.clone(),
                convention: convention.clone(),
            },
            VolQuote::CapFloorVol {
                id,
                expiry,
                strike,
                vol,
                quote_type,
                is_cap,
                convention,
            } => VolQuote::CapFloorVol {
                id: id.clone(),
                expiry: *expiry,
                strike: *strike,
                vol: vol + vol_bump,
                quote_type: quote_type.clone(),
                is_cap: *is_cap,
                convention: convention.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::conventions::ids::CapFloorConventionId;
    use time::macros::date;

    #[test]
    fn parse_vol_quote_type_accepts_standard_vocabulary() {
        for s in ["normal", "Normal", "NORMAL", "bachelier", "Bachelier"] {
            assert_eq!(
                parse_vol_quote_type(s).expect("normal vocabulary"),
                VolQuoteType::Normal,
                "'{s}' should parse as Normal"
            );
        }
        for s in [
            "lognormal",
            "log_normal",
            "black",
            "Black",
            "black_lognormal",
            "BLACK_LOGNORMAL",
        ] {
            assert_eq!(
                parse_vol_quote_type(s).expect("lognormal vocabulary"),
                VolQuoteType::BlackLognormal,
                "'{s}' should parse as BlackLognormal"
            );
        }
    }

    /// Regression: unknown strings must fail closed, never silently default
    /// to lognormal. "implied_vol" and moneyness labels are the historical
    /// offenders.
    #[test]
    fn parse_vol_quote_type_rejects_unknown_strings() {
        for s in ["implied_vol", "ATM", "ATM-50", "nrmal", "", "vol"] {
            let result = parse_vol_quote_type(s);
            assert!(result.is_err(), "'{s}' should be rejected");
        }
    }

    #[test]
    fn cap_floor_vol_quote_bumps_absolute_vol() {
        let quote = VolQuote::CapFloorVol {
            id: QuoteId::new("USD-CAP-VOL-20310506-0.0366561"),
            expiry: date!(2031 - 05 - 06),
            strike: 0.0366561,
            vol: 0.0088,
            quote_type: "normal".to_string(),
            is_cap: true,
            convention: CapFloorConventionId::new("USD-SOFR-CAP"),
        };

        let bumped = quote.bump_vol_absolute(0.0001);

        match bumped {
            VolQuote::CapFloorVol { vol, .. } => {
                assert!((vol - 0.0089).abs() < 1e-12);
            }
            other => panic!("unexpected bumped quote: {other:?}"),
        }
    }
}
