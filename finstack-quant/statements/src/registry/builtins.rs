//! Built-in financial metrics.
//!
//! This module provides access to standard financial metrics that are
//! bundled with the crate. Metric JSON sources are embedded at compile time
//! from `data/metrics` and are exposed via a small helper that is used by
//! [`Registry::load_builtins()`](crate::registry::Registry::load_builtins).
//!
//! Metrics are organized into namespaces:
//! - `fin.*` - Standard financial metrics
//!   - `fin_basic.json` - Basic metrics (gross_profit, net_income, etc.)
//!   - `fin_margins.json` - Margin calculations
//!   - `fin_returns.json` - Return metrics (ROE, ROA, ROIC, etc.)
//!   - `fin_leverage.json` - Leverage ratios
//!
//! ## Input convention (BREAKING CHANGE, 2026-07)
//!
//! All bundled P&L and return metrics assume the `opex` input **excludes**
//! depreciation and amortization, which are supplied as the separate
//! `depreciation` and `amortization` nodes:
//!
//! - `fin.ebitda = revenue - cogs - opex` (no D&A add-back)
//! - `fin.operating_income = fin.ebit = revenue - cogs - opex - depreciation - amortization`
//! - `fin.ebt`, `fin.net_income`, `fin.roe`, `fin.roa`, `fin.roic`, `fin.roce`
//!   subtract D&A explicitly on the same basis.
//!
//! **Migration:** models that previously fed `opex` *including* D&A must
//! split D&A out into the `depreciation` / `amortization` nodes, otherwise
//! D&A is double-counted and EBITDA/EBIT/net income are understated. Under
//! the old formulas EBITDA added D&A back onto an `opex` that was assumed to
//! contain it; the formulas above are the standard presentation and make
//! `ebitda - D - A == ebit == operating_income` articulate exactly.
//!
//! ## Usage
//!
//! Built-in metrics are loaded via [`Registry::load_builtins()`](crate::registry::Registry::load_builtins)
//! from compile-time embedded JSON sources, so packaged binaries and WASM
//! builds do not require a runtime `data/metrics` directory.

use crate::error::Result;

/// Discover and load all bundled metric registry JSON files.
///
/// Built-in metrics are embedded at compile time for all targets so packaged
/// binaries do not depend on a source-tree `data/metrics` directory at runtime.
pub(crate) fn builtin_metric_sources() -> Result<Vec<String>> {
    let files: &[(&str, &str)] = &[
        (
            "fin_basic.json",
            include_str!("../../data/metrics/fin_basic.json"),
        ),
        (
            "fin_leverage.json",
            include_str!("../../data/metrics/fin_leverage.json"),
        ),
        (
            "fin_margins.json",
            include_str!("../../data/metrics/fin_margins.json"),
        ),
        (
            "fin_returns.json",
            include_str!("../../data/metrics/fin_returns.json"),
        ),
    ];

    let mut discovered: Vec<(String, String)> = files
        .iter()
        .map(|(name, contents)| (name.to_string(), contents.to_string()))
        .collect();

    // Ensure deterministic ordering regardless of list order
    discovered.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(discovered
        .into_iter()
        .map(|(_, contents)| contents)
        .collect())
}
