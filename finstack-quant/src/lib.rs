#![forbid(unsafe_code)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]
#![doc(test(attr(allow(clippy::expect_used))))]

//! Umbrella crate for the **Finstack Quant** quantitative-finance toolkit.
//!
//! Re-exports each sub-crate so downstream consumers can reach the full API
//! through a single dependency:
//!
//! | Re-export          | Sub-crate                         |
//! |--------------------|-----------------------------------|
//! | `core`             | [`finstack_quant_core`]                 |
//! | `analytics`        | [`finstack_quant_analytics`]            |
//! | `attribution`      | [`finstack_quant_attribution`]          |
//! | `cashflows`        | [`finstack_quant_cashflows`]            |
//! | `covenants`        | [`finstack_quant_covenants`]            |
//! | `factor_model`     | [`finstack_quant_factor_model`]         |
//! | `features`         | [`finstack_quant_features`]             |
//! | `margin`           | [`finstack_quant_margin`]               |
//! | `monte_carlo`      | [`finstack_quant_monte_carlo`]          |
//! | `valuations`       | [`finstack_quant_valuations`]           |
//! | `statements`       | [`finstack_quant_statements`]           |
//! | `statements_analytics` | [`finstack_quant_statements_analytics`] |
//! | `portfolio`        | [`finstack_quant_portfolio`]            |
//! | `scenarios`        | [`finstack_quant_scenarios`]            |

pub use finstack_quant_analytics as analytics;
pub use finstack_quant_attribution as attribution;
pub use finstack_quant_cashflows as cashflows;
pub use finstack_quant_core as core;
pub use finstack_quant_covenants as covenants;
pub use finstack_quant_factor_model as factor_model;
pub use finstack_quant_features as features;
pub use finstack_quant_margin as margin;
pub use finstack_quant_monte_carlo as monte_carlo;
pub use finstack_quant_portfolio as portfolio;
pub use finstack_quant_scenarios as scenarios;
pub use finstack_quant_statements as statements;
pub use finstack_quant_statements_analytics as statements_analytics;
pub use finstack_quant_valuations as valuations;

#[cfg(test)]
mod umbrella_surface {
    //! The umbrella crate must expose every domain crate so downstream users can
    //! reach the full API through a single dependency.
    //!
    //! Regression: attribution was omitted entirely, which made portfolio and
    //! scenario APIs naming attribution types unusable through the umbrella.

    fn assert_type<T>() {
        let _ = std::mem::size_of::<T>();
    }

    /// Attribution types named in public portfolio and scenario signatures must be
    /// reachable through the umbrella.
    #[test]
    fn umbrella_exposes_attribution_types_used_by_portfolio_and_scenarios() {
        assert_type::<crate::attribution::AttributionMethod>();
        assert_type::<crate::attribution::PnlAttribution>();
        assert_type::<crate::attribution::AttributionFactor>();
    }
}
