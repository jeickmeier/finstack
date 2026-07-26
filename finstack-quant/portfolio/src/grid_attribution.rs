//! Hierarchical duration-cell x sector grid attribution (Dynkin, Hyman &
//! Vankudre 1998, Appendix A).
//!
//! Lehman's grid attribution splits benchmark-relative fixed-income
//! performance into a two-level hierarchy: coarse duration cells (the
//! "curve" axis) crossed with sectors *within* each cell (the "sector" axis),
//! plus a security-selection residual. Unlike [`crate::fi_attribution`]
//! (which decomposes *component* returns — carry/treasury/spread/selection —
//! within a flat sector grouping), this module decomposes the *positioning*
//! of weight across a two-level cell x sector grid using only weights and
//! total returns.
//!
//! # Notation
//!
//! For cell `t` and sector `s` within it, on side `k ∈ {P, B}` (portfolio /
//! benchmark):
//!
//! ```text
//! x_t^k   = cell weight            = Σ_s y_st^k
//! y_st^k  = (cell, sector) weight  = Σ_j w_j   (positions in cell t, sector s)
//! z_st^k  = within-cell share      = y_st^k / x_t^k
//! r_st^k  = (cell, sector) return  = (Σ_j w_j r_j) / y_st^k
//! r_t^k   = cell return            = Σ_s z_st^k r_st^k = (Σ_s y_st^k r_st^k) / x_t^k
//! r^k     = side return            = Σ_t x_t^k r_t^k
//! ```
//!
//! # Formulas
//!
//! ```text
//! e^curve_t     = (x_t^P − x_t^B)(r_t^B − r^B)
//! e^sector_st   = x_t^P (z_st^P − z_st^B)(r_st^B − r_t^B)
//! e^security_st = y_st^P (r_st^P − r_st^B)
//! Σ_t e^curve_t + Σ_st e^sector_st + Σ_st e^security_st ≡ r^P − r^B
//! ```
//!
//! # Telescoping identity
//!
//! The three sums telescope to the active return exactly, given per-side
//! weight normalization (`Σ_t x_t^k = 1`) and within-cell share normalization
//! (`Σ_s z_st^k = 1` for every cell present on side `k`):
//!
//! ```text
//! Σ_st e^security_st = Σ_st y_st^P r_st^P − Σ_st y_st^P r_st^B
//!                     = r^P − Σ_st y_st^P r_st^B
//! Σ_st e^sector_st    = Σ_st y_st^P r_st^B − Σ_t x_t^P r_t^B      (Σ_s z_st^P = Σ_s z_st^B = 1)
//! Σ_t  e^curve_t      = Σ_t x_t^P r_t^B − r^B                    (Σ_t x_t^P = Σ_t x_t^B = 1)
//! ------------------------------------------------------------------------
//! Sum                 = r^P − r^B
//! ```
//!
//! Exactness therefore *requires* the per-side weight-sum validation and
//! within-cell share normalization enforced by [`grid_attribution`] — they
//! are not cosmetic input checks, they are the hypotheses of the identity
//! above.
//!
//! # Out-of-benchmark fallbacks
//!
//! A bucket with portfolio positions but no benchmark counterpart has an
//! undefined per-unit benchmark rate. Lehman's convention assigns it a
//! return "typical of the sector" — the portfolio's own rate — with zero
//! benchmark weight, so its selection effect collapses to zero and the whole
//! effect flows to the coarser level:
//!
//! - **Cell absent from the benchmark** (`x_t^B = 0`, i.e. no benchmark
//!   positions in the cell at all): `r_t^B := r_t^P`, and every sector's
//!   benchmark share/return within the cell falls back to the portfolio's
//!   (`z_st^B := z_st^P`, `r_st^B := r_st^P`). Both `e^sector_st` and
//!   `e^security_st` collapse to zero for every sector in the cell; the
//!   entire effect surfaces as `e^curve_t = x_t^P (r_t^P − r^B)`.
//! - **(Cell, sector) absent from the benchmark but the cell is present**
//!   (`x_t^B ≠ 0`, `y_st^B = 0`): `z_st^B` is naturally `0` (no fallback
//!   needed there), but `r_st^B` has no data, so `r_st^B := r_st^P`.
//!   `e^security_st` collapses to zero; the effect surfaces as
//!   `e^sector_st = x_t^P z_st^P (r_st^P − r_t^B)`.
//!
//! A bucket with *no* portfolio positions needs no fallback at all: every
//! term above carries a portfolio-side weight factor (`x_t^P` or `y_st^P`)
//! that is already zero.
//!
//! # Fail-closed validation
//!
//! - Each side's weights must sum to `1.0` within `±1e-6`.
//! - A bucket (cell, or cell+sector) that has positions on a side but nets
//!   to *exactly* zero weight with non-zero gross weight (e.g. a long/short
//!   pair in the same bucket) has an undefined per-unit rate. Rather than
//!   silently zeroing its effects — which would leave its non-zero
//!   contribution to the side return unexplained and break the telescoping
//!   identity — [`grid_attribution`] rejects it, naming the bucket and the
//!   side.
//! - A bucket that nets to a *non-zero* (including negative / net-short)
//!   weight is ordinary input and is attributed normally by dividing through
//!   the net weight, not the gross weight.
//!
//! # Ordering
//!
//! Cells and, within each cell, sectors are emitted in first-appearance
//! order: portfolio positions are scanned before benchmark positions, so
//! portfolio-seen buckets sort first, followed by benchmark-only buckets
//! ([`indexmap::IndexMap`] preserves insertion order deterministically).
//!
//! # References
//!
//! * Dynkin, L., Hyman, J., & Vankudre, P. (1998). "Attribution of Portfolio
//!   Performance Relative to an Index." Lehman Brothers Fixed Income
//!   Research, March 1998, Appendix A — source of the curve/sector/selection
//!   grid decomposition and the out-of-benchmark fallback convention used
//!   above. `docs/REFERENCES.md#dynkin-hyman-vankudre-1998`

use crate::error::{Error, Result};
use finstack_quant_core::math::summation::NeumaierAccumulator;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Tolerance for the requirement that weights sum to 1.0 on each side.
const WEIGHT_TOLERANCE: f64 = 1e-6;

/// One position (or pre-aggregated bucket) in a duration-cell x sector grid,
/// for one period and one side (portfolio or benchmark).
///
/// Weights are fractions of the whole side and must sum to `1.0` (within
/// [`WEIGHT_TOLERANCE`]) across all positions on that side. Returns are
/// decimals (`0.02` = 2 %).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridPosition {
    /// Duration-cell label. Cell labels may come from
    /// [`crate::excess_return::duration_cell_label`], but any string key is
    /// accepted — this module has no dependency on how cells were built.
    pub cell: String,
    /// Sector bucket label within the cell.
    pub sector: String,
    /// Weight as a fraction of the whole side at period start (decimal).
    pub weight: f64,
    /// Realized total return for the period (decimal).
    pub total_return: f64,
}

/// Per-cell curve (duration-cell positioning) effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridCellEffect {
    /// Duration-cell label.
    pub cell: String,
    /// Portfolio net weight in the cell, `x_t^P`.
    pub portfolio_weight: f64,
    /// Benchmark net weight in the cell, `x_t^B`.
    pub benchmark_weight: f64,
    /// Benchmark cell return, `r_t^B`. For a cell absent from the benchmark
    /// this is the out-of-benchmark fallback (`r_t^P`); see the module docs.
    pub benchmark_cell_return: f64,
    /// Curve effect `(x_t^P − x_t^B)(r_t^B − r^B)`.
    pub curve_effect: f64,
}

/// Per-(cell, sector) within-cell sector-allocation effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridSectorEffect {
    /// Duration-cell label.
    pub cell: String,
    /// Sector label within the cell.
    pub sector: String,
    /// Allocation effect `x_t^P (z_st^P − z_st^B)(r_st^B − r_t^B)`.
    pub allocation_effect: f64,
}

/// Per-(cell, sector) security-selection effect.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridSelectionEffect {
    /// Duration-cell label.
    pub cell: String,
    /// Sector label within the cell.
    pub sector: String,
    /// Selection effect `y_st^P (r_st^P − r_st^B)`.
    pub selection_effect: f64,
}

/// Single-period hierarchical grid attribution result.
///
/// This type is *input-reachable*: multi-period linking (Task 5) consumes a
/// slice of these, and the Python/WASM bindings deserialize them from JSON.
/// It therefore denies unknown fields, like the other inbound types in this
/// module, so a misspelled or stale key fails closed instead of being
/// silently dropped.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridAttributionResult {
    /// Portfolio total return `r^P = Σ_t x_t^P r_t^P`.
    pub portfolio_return: f64,
    /// Benchmark total return `r^B = Σ_t x_t^B r_t^B`.
    pub benchmark_return: f64,
    /// Active return `r^P − r^B`.
    pub active_return: f64,
    /// Per-cell curve effects, in first-appearance order.
    pub curve_effects: Vec<GridCellEffect>,
    /// Per-(cell, sector) allocation effects, in first-appearance order
    /// (cells, then sectors within each cell).
    pub sector_effects: Vec<GridSectorEffect>,
    /// Per-(cell, sector) selection effects, in the same order as
    /// `sector_effects`.
    pub selection_effects: Vec<GridSelectionEffect>,
    /// Sum of the curve effects.
    pub total_curve: f64,
    /// Sum of the sector allocation effects.
    pub total_sector: f64,
    /// Sum of the selection effects.
    pub total_selection: f64,
}

/// Net/gross weight plus weighted-return accumulation for one bucket (a cell,
/// or a (cell, sector) pair) on one side.
///
/// `weight` is the *net* bucket weight (long minus short) and `abs_weight`
/// the gross weight; the pair distinguishes "bucket absent from this side"
/// (`abs_weight == 0`) from "bucket present with offsetting positions"
/// (`abs_weight > 0`, `weight == 0`), which [`check_zero_net_weight`]
/// rejects.
#[derive(Clone, Copy, Default)]
struct BucketAgg {
    weight: f64,
    abs_weight: f64,
    weighted_return: f64,
}

impl BucketAgg {
    /// Fold one position's weight/return into this bucket.
    fn add(&mut self, weight: f64, total_return: f64) {
        self.weight += weight;
        self.abs_weight += weight.abs();
        self.weighted_return += weight * total_return;
    }

    /// Weighted-average return `weighted_return / weight`.
    ///
    /// Returns `0.0` only when the bucket carries no net weight, i.e. it is
    /// either absent from this side or was rejected by
    /// [`check_zero_net_weight`] before this is ever consulted for a real
    /// contribution. The bare (non-`.abs()`) comparison against `weight`
    /// elsewhere in this module is intentional: net-short buckets
    /// (`weight < 0`) are legal and must still divide through.
    fn rate(&self) -> f64 {
        if self.weight != 0.0 {
            self.weighted_return / self.weight
        } else {
            0.0
        }
    }
}

/// Per-(cell, sector) aggregates for both sides, keyed by sector within a
/// cell, in first-appearance order.
type SectorMap = IndexMap<String, (BucketAgg, BucketAgg)>;

/// Validate finiteness of a position's numeric fields.
fn validate_position(p: &GridPosition, side: &str) -> Result<()> {
    for (name, value) in [("weight", p.weight), ("total_return", p.total_return)] {
        if !value.is_finite() {
            return Err(Error::invalid_input(format!(
                "Grid attribution {side} input '{name}' for cell '{}' sector '{}' must be \
                 finite (got {value})",
                p.cell, p.sector
            )));
        }
    }
    Ok(())
}

/// Fail closed on a bucket that is present on a side but nets to exactly
/// zero weight (a long/short pair, a hedge against a cash position in the
/// same bucket).
///
/// Such a bucket still contributes `Σ_j w_j r_j ≠ 0` to the side total, but
/// its per-unit rate `weighted_return / weight` is undefined, so every
/// effect derived from it would be forced to zero while the contribution
/// stayed in the side return — breaking the telescoping identity while
/// `active_return` still ties out against performance data.
fn check_zero_net_weight(bucket: &str, agg: &BucketAgg, side_name: &str) -> Result<()> {
    if agg.weight == 0.0 && agg.abs_weight > 0.0 {
        return Err(Error::invalid_input(format!(
            "{side_name} bucket '{bucket}' has offsetting positions netting to exactly zero \
             weight (gross weight {}); a zero-net-weight bucket cannot be attributed because \
             its per-unit rate (weighted return / weight) is undefined. Split the offsetting \
             positions into distinct buckets, or net them into a single position with non-zero \
             weight.",
            agg.abs_weight
        )));
    }
    Ok(())
}

/// Accumulate one side's positions into the cell x sector grid.
///
/// Returns the side's total return `Σ_j w_j r_j` and validates that the
/// side's weights sum to `1.0` within [`WEIGHT_TOLERANCE`].
fn aggregate_side(
    positions: &[GridPosition],
    cells: &mut IndexMap<String, SectorMap>,
    is_portfolio: bool,
) -> Result<f64> {
    let (side, side_name) = if is_portfolio {
        ("portfolio", "Portfolio")
    } else {
        ("benchmark", "Benchmark")
    };
    let mut sum_w = NeumaierAccumulator::new();
    let mut sum_r = NeumaierAccumulator::new();

    for p in positions {
        validate_position(p, side)?;
        sum_w.add(p.weight);
        sum_r.add(p.weight * p.total_return);

        let sector_map = cells.entry(p.cell.clone()).or_default();
        let entry = sector_map.entry(p.sector.clone()).or_default();
        let agg = if is_portfolio {
            &mut entry.0
        } else {
            &mut entry.1
        };
        agg.add(p.weight, p.total_return);
    }

    let total_w = sum_w.total();
    if (total_w - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(Error::invalid_input(format!(
            "{side_name} weights must sum to 1.0 (got {total_w})"
        )));
    }

    Ok(sum_r.total())
}

/// Compute a single-period hierarchical duration-cell x sector grid
/// attribution (Dynkin, Hyman & Vankudre 1998, Appendix A).
///
/// Decomposes the active return into a curve effect per duration cell, a
/// within-cell sector-allocation effect, and a security-selection residual
/// per (cell, sector) — see the module docs for the exact formulas, the
/// out-of-benchmark fallback convention, and a proof that the three sums
/// telescope exactly to the active return.
///
/// # Arguments
///
/// * `portfolio` - Portfolio position/bucket snapshots; weights must sum to
///   `1.0` within `±1e-6`.
/// * `benchmark` - Benchmark snapshots; weights must sum to `1.0` within
///   `±1e-6`.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if any weight or return is non-finite,
/// either side's weights don't sum to `1.0` (±1e-6), or a (cell) or
/// (cell, sector) bucket has positions on a side but nets to exactly zero
/// weight with non-zero gross weight.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::grid_attribution::{grid_attribution, GridPosition};
///
/// let pos = |cell: &str, sector: &str, weight: f64, total_return: f64| GridPosition {
///     cell: cell.into(),
///     sector: sector.into(),
///     weight,
///     total_return,
/// };
/// let portfolio = vec![
///     pos("0.0-3.0", "GOVT", 0.5, 0.012),
///     pos("0.0-3.0", "CORP", 0.5, 0.020),
/// ];
/// let benchmark = vec![
///     pos("0.0-3.0", "GOVT", 0.6, 0.010),
///     pos("0.0-3.0", "CORP", 0.4, 0.018),
/// ];
/// let result = grid_attribution(&portfolio, &benchmark)?;
/// let reconstructed = result.total_curve + result.total_sector + result.total_selection;
/// assert!((reconstructed - result.active_return).abs() < 1e-12);
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
pub fn grid_attribution(
    portfolio: &[GridPosition],
    benchmark: &[GridPosition],
) -> Result<GridAttributionResult> {
    // Union of cells (and, within each, sectors) in first-appearance order:
    // portfolio positions are scanned before benchmark positions.
    let mut cells: IndexMap<String, SectorMap> = IndexMap::new();
    let portfolio_return = aggregate_side(portfolio, &mut cells, true)?;
    let benchmark_return = aggregate_side(benchmark, &mut cells, false)?;
    let active_return = portfolio_return - benchmark_return;

    // Cell-level totals, derived by summing the per-sector aggregates within
    // each cell. Iteration order matches `cells` exactly, since it is built
    // by iterating `cells` itself.
    let mut cell_totals: IndexMap<String, (BucketAgg, BucketAgg)> = IndexMap::new();
    for (cell, sector_map) in &cells {
        let mut p = BucketAgg::default();
        let mut b = BucketAgg::default();
        for (_, (sp, sb)) in sector_map {
            p.weight += sp.weight;
            p.abs_weight += sp.abs_weight;
            p.weighted_return += sp.weighted_return;
            b.weight += sb.weight;
            b.abs_weight += sb.abs_weight;
            b.weighted_return += sb.weighted_return;
        }
        cell_totals.insert(cell.clone(), (p, b));
    }

    // Fail closed on zero-net-weight buckets before computing any effects,
    // at both the cell level and the (cell, sector) level.
    for (cell, (p, b)) in &cell_totals {
        check_zero_net_weight(cell, p, "Portfolio")?;
        check_zero_net_weight(cell, b, "Benchmark")?;
    }
    for (cell, sector_map) in &cells {
        for (sector, (sp, sb)) in sector_map {
            let bucket = format!("{cell}/{sector}");
            check_zero_net_weight(&bucket, sp, "Portfolio")?;
            check_zero_net_weight(&bucket, sb, "Benchmark")?;
        }
    }

    let mut total_curve = NeumaierAccumulator::new();
    let mut total_sector = NeumaierAccumulator::new();
    let mut total_selection = NeumaierAccumulator::new();
    let mut curve_effects = Vec::with_capacity(cell_totals.len());
    let mut sector_effects = Vec::new();
    let mut selection_effects = Vec::new();

    for ((cell, sector_map), (_, (p, b))) in cells.iter().zip(cell_totals.iter()) {
        let x_p = p.weight;
        let x_b = b.weight;
        // A cell absent from the benchmark has zero net *and* zero gross
        // weight there (the zero-net-weight check above already ruled out
        // "present but netting to zero"), so `x_b == 0.0` is exactly the
        // out-of-benchmark case from the module docs.
        let cell_absent_from_benchmark = x_b == 0.0;

        let r_t_p = p.rate();
        let r_t_b = if cell_absent_from_benchmark {
            r_t_p
        } else {
            b.rate()
        };

        let curve_effect = (x_p - x_b) * (r_t_b - benchmark_return);
        total_curve.add(curve_effect);
        curve_effects.push(GridCellEffect {
            cell: cell.clone(),
            portfolio_weight: x_p,
            benchmark_weight: x_b,
            benchmark_cell_return: r_t_b,
            curve_effect,
        });

        for (sector, (sp, sb)) in sector_map {
            let z_p = if x_p != 0.0 { sp.weight / x_p } else { 0.0 };
            let r_st_p = sp.rate();

            let (z_b, r_st_b) = if cell_absent_from_benchmark {
                (z_p, r_st_p)
            } else if sb.weight == 0.0 {
                // (cell, sector) absent from the benchmark, but the cell is
                // present: z_st^B = 0 is already correct (no fallback
                // needed there); only the return needs a fallback.
                (0.0, r_st_p)
            } else {
                (sb.weight / x_b, sb.rate())
            };

            let allocation_effect = x_p * (z_p - z_b) * (r_st_b - r_t_b);
            let selection_effect = sp.weight * (r_st_p - r_st_b);

            total_sector.add(allocation_effect);
            total_selection.add(selection_effect);

            sector_effects.push(GridSectorEffect {
                cell: cell.clone(),
                sector: sector.clone(),
                allocation_effect,
            });
            selection_effects.push(GridSelectionEffect {
                cell: cell.clone(),
                sector: sector.clone(),
                selection_effect,
            });
        }
    }

    Ok(GridAttributionResult {
        portfolio_return,
        benchmark_return,
        active_return,
        curve_effects,
        sector_effects,
        selection_effects,
        total_curve: total_curve.total(),
        total_sector: total_sector.total(),
        total_selection: total_selection.total(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(cell: &str, sector: &str, weight: f64, total_return: f64) -> GridPosition {
        GridPosition {
            cell: cell.into(),
            sector: sector.into(),
            weight,
            total_return,
        }
    }

    fn close(actual: f64, expected: f64, tol: f64, label: &str) {
        assert!(
            (actual - expected).abs() < tol,
            "{label}: got {actual}, expected {expected} (tol {tol})"
        );
    }

    /// Hand-verified golden fixture (re-derived twice by hand):
    ///
    /// ```text
    /// Benchmark: C1 "0.0-3.0": GOVT 0.30@0.010, CORP 0.20@0.020  => x1=0.50, r1=0.014
    ///            C2 "3.0-6.0": GOVT 0.25@0.030, CORP 0.25@0.040  => x2=0.50, r2=0.035
    ///            r^B = 0.0245
    /// Portfolio: C1: GOVT 0.20@0.012, CORP 0.20@0.025            => x1=0.40
    ///            C2: GOVT 0.30@0.028, CORP 0.30@0.045            => x2=0.60
    ///            r^P = 0.0293, active = 0.0048
    /// curve:   C1 (0.40-0.50)(0.014-0.0245)=+0.00105 ; C2 (0.60-0.50)(0.035-0.0245)=+0.00105
    /// sector:  C1 0.40*[(0.5-0.6)(0.010-0.014)+(0.5-0.4)(0.020-0.014)] = 0.0004 ; C2 = 0
    /// select:  0.20(0.002)+0.20(0.005)+0.30(-0.002)+0.30(0.005) = 0.0023
    /// totals:  0.0021 + 0.0004 + 0.0023 = 0.0048 (equals active return)
    /// ```
    #[test]
    fn grid_attribution_matches_hand_derived_golden() {
        let p = |cell: &str, sector: &str, w: f64, r: f64| GridPosition {
            cell: cell.into(),
            sector: sector.into(),
            weight: w,
            total_return: r,
        };
        let portfolio = vec![
            p("0.0-3.0", "GOVT", 0.20, 0.012),
            p("0.0-3.0", "CORP", 0.20, 0.025),
            p("3.0-6.0", "GOVT", 0.30, 0.028),
            p("3.0-6.0", "CORP", 0.30, 0.045),
        ];
        let benchmark = vec![
            p("0.0-3.0", "GOVT", 0.30, 0.010),
            p("0.0-3.0", "CORP", 0.20, 0.020),
            p("3.0-6.0", "GOVT", 0.25, 0.030),
            p("3.0-6.0", "CORP", 0.25, 0.040),
        ];
        let r = grid_attribution(&portfolio, &benchmark).expect("valid golden inputs");
        let c = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} vs {b}");
        c(r.portfolio_return, 0.0293);
        c(r.benchmark_return, 0.0245);
        c(r.active_return, 0.0048);
        c(r.total_curve, 0.0021);
        c(r.total_sector, 0.0004);
        c(r.total_selection, 0.0023);
        c(r.curve_effects[0].curve_effect, 0.00105);
        c(r.curve_effects[1].curve_effect, 0.00105);
        c(
            r.total_curve + r.total_sector + r.total_selection,
            r.active_return,
        );
    }

    /// A cell the portfolio holds but the benchmark entirely lacks falls
    /// back to `r_t^B := r_t^P`: its selection effect must be exactly zero
    /// and the whole effect must surface as curve, while the identity still
    /// closes exactly.
    #[test]
    fn out_of_benchmark_cell_flows_to_curve_with_zero_selection() {
        let portfolio = vec![
            pos("A", "GOVT", 0.5, 0.02),
            pos("B", "CORP", 0.5, 0.03), // benchmark has no "B" cell at all
        ];
        let benchmark = vec![pos("A", "GOVT", 1.0, 0.01)];

        let r = grid_attribution(&portfolio, &benchmark).expect("valid inputs");

        let reconstructed = r.total_curve + r.total_sector + r.total_selection;
        close(reconstructed, r.active_return, 1e-12, "reconciliation");

        let cell_b = r
            .curve_effects
            .iter()
            .find(|e| e.cell == "B")
            .expect("cell B present");
        close(
            cell_b.benchmark_weight,
            0.0,
            1e-15,
            "cell B benchmark weight",
        );
        close(
            cell_b.benchmark_cell_return,
            0.03,
            1e-15,
            "cell B r_t^B fallback = r_t^P",
        );
        // (0.5 - 0.0) * (0.03 - 0.01) = 0.01, the entire out-of-benchmark
        // effect surfaces as curve.
        close(cell_b.curve_effect, 0.01, 1e-12, "cell B curve effect");

        let sel_b = r
            .selection_effects
            .iter()
            .find(|e| e.cell == "B" && e.sector == "CORP")
            .expect("selection entry for B/CORP");
        close(
            sel_b.selection_effect,
            0.0,
            1e-15,
            "out-of-benchmark selection is zero",
        );

        let alloc_b = r
            .sector_effects
            .iter()
            .find(|e| e.cell == "B" && e.sector == "CORP")
            .expect("allocation entry for B/CORP");
        close(
            alloc_b.allocation_effect,
            0.0,
            1e-15,
            "single-sector cell allocation is zero",
        );
    }

    /// C1 GOVT nets to exactly zero on the portfolio side (a +0.5/-0.5
    /// offsetting pair) despite carrying real positions; the per-unit rate
    /// is undefined and must be rejected, naming the cell, sector and side.
    #[test]
    fn zero_net_weight_bucket_with_positions_fails_closed() {
        let portfolio = vec![
            pos("0.0-3.0", "GOVT", 0.5, 0.01),
            pos("0.0-3.0", "GOVT", -0.5, 0.02),
            pos("0.0-3.0", "CORP", 1.0, 0.03),
        ];
        let benchmark = vec![pos("0.0-3.0", "CORP", 1.0, 0.025)];

        let err = grid_attribution(&portfolio, &benchmark)
            .expect_err("zero-net-weight bucket must be rejected");
        let message = err.to_string();
        assert!(
            message.contains("0.0-3.0"),
            "error must name the cell: {message}"
        );
        assert!(
            message.contains("GOVT"),
            "error must name the sector: {message}"
        );
        assert!(
            message.contains("Portfolio"),
            "error must name the side: {message}"
        );
    }

    /// A bucket netting to a non-zero, negative (net-short) weight must be
    /// attributed by dividing through the net weight, not zeroed. Pins the
    /// non-`.abs()` guard in [`BucketAgg::rate`] (the Campisi F4 lesson:
    /// dropping this silently zeroes every net-short bucket).
    #[test]
    fn net_short_bucket_is_attributed_not_zeroed() {
        let portfolio = vec![pos("A", "CORE", 1.2, 0.02), pos("A", "SHORT", -0.2, 0.05)];
        let benchmark = vec![pos("A", "CORE", 0.9, 0.015), pos("A", "SHORT", 0.1, 0.04)];

        let r = grid_attribution(&portfolio, &benchmark).expect("net-short is legal");

        let alloc_short = r
            .sector_effects
            .iter()
            .find(|e| e.cell == "A" && e.sector == "SHORT")
            .expect("SHORT allocation entry");
        let sel_short = r
            .selection_effects
            .iter()
            .find(|e| e.cell == "A" && e.sector == "SHORT")
            .expect("SHORT selection entry");

        // Hand-derived: x_p = x_b = 1.0 (single cell), z_p^SHORT = -0.2,
        // z_b^SHORT = 0.1, r_t^B = 0.0175, r_st^B^SHORT = 0.04.
        assert!(
            alloc_short.allocation_effect.abs() > 1e-6,
            "must not be silently zeroed"
        );
        assert!(
            sel_short.selection_effect.abs() > 1e-6,
            "must not be silently zeroed"
        );
        close(
            alloc_short.allocation_effect,
            -0.00675,
            1e-12,
            "SHORT allocation effect",
        );
        close(
            sel_short.selection_effect,
            -0.002,
            1e-12,
            "SHORT selection effect",
        );

        let reconstructed = r.total_curve + r.total_sector + r.total_selection;
        close(reconstructed, r.active_return, 1e-12, "reconciliation");
    }

    /// Three cells x three sectors, irregular weights, a negative return,
    /// and two one-sided (cell, sector) buckets (one portfolio-only, one
    /// benchmark-only) within cells that are otherwise present on both
    /// sides. The telescoping identity must still close to float precision.
    #[test]
    fn adversarial_three_cell_fixture_reconciles_algebraically() {
        let portfolio = vec![
            pos("0-2", "GOVT", 0.15, 0.01),
            pos("0-2", "IG", 0.10, -0.02), // negative return
            pos("2-5", "GOVT", 0.20, 0.015),
            pos("2-5", "HY", 0.05, 0.04), // portfolio-only bucket in "2-5"
            pos("5-10", "IG", 0.30, 0.03), // portfolio-only bucket in "5-10"
            pos("5-10", "HY", 0.20, 0.06),
        ];
        let benchmark = vec![
            pos("0-2", "GOVT", 0.25, 0.008),
            pos("0-2", "IG", 0.15, -0.01),
            pos("2-5", "GOVT", 0.10, 0.012),
            pos("2-5", "IG", 0.10, 0.02), // benchmark-only bucket in "2-5"
            pos("5-10", "HY", 0.40, 0.05),
        ];

        let r = grid_attribution(&portfolio, &benchmark).expect("valid adversarial inputs");
        let reconstructed = r.total_curve + r.total_sector + r.total_selection;
        assert!(
            (reconstructed - r.active_return).abs() < 1e-15,
            "reconstructed {reconstructed} vs active {}",
            r.active_return
        );
    }

    /// Pins [`WEIGHT_TOLERANCE`] itself, in both directions: `1 + 5e-7` must
    /// be accepted, `1 + 2e-6` must be rejected.
    #[test]
    fn weight_tolerance_is_pinned_at_1e_minus_6_both_directions() {
        let base_portfolio = vec![
            pos("0.0-3.0", "GOVT", 0.20, 0.012),
            pos("0.0-3.0", "CORP", 0.20, 0.025),
            pos("3.0-6.0", "GOVT", 0.30, 0.028),
            pos("3.0-6.0", "CORP", 0.30, 0.045),
        ];
        let benchmark = vec![
            pos("0.0-3.0", "GOVT", 0.30, 0.010),
            pos("0.0-3.0", "CORP", 0.20, 0.020),
            pos("3.0-6.0", "GOVT", 0.25, 0.030),
            pos("3.0-6.0", "CORP", 0.25, 0.040),
        ];

        let mut just_over = base_portfolio.clone();
        just_over[0].weight += 2e-6;
        let err = grid_attribution(&just_over, &benchmark)
            .expect_err("1 + 2e-6 must be rejected at a 1e-6 tolerance");
        assert!(err.to_string().contains("Portfolio weights"), "{err}");

        let mut just_under = base_portfolio;
        just_under[0].weight += 5e-7;
        assert!(
            grid_attribution(&just_under, &benchmark).is_ok(),
            "1 + 5e-7 must be accepted at a 1e-6 tolerance"
        );
    }

    #[test]
    fn grid_attribution_result_serde_round_trips() {
        let portfolio = vec![pos("0.0-3.0", "GOVT", 1.0, 0.012)];
        let benchmark = vec![pos("0.0-3.0", "GOVT", 1.0, 0.010)];
        let r = grid_attribution(&portfolio, &benchmark).expect("valid inputs");

        let json = serde_json::to_string(&r).expect("serializes");
        let round_tripped: GridAttributionResult =
            serde_json::from_str(&json).expect("deserializes");

        close(
            round_tripped.portfolio_return,
            r.portfolio_return,
            1e-15,
            "portfolio_return",
        );
        close(
            round_tripped.benchmark_return,
            r.benchmark_return,
            1e-15,
            "benchmark_return",
        );
        close(
            round_tripped.active_return,
            r.active_return,
            1e-15,
            "active_return",
        );
        close(
            round_tripped.total_curve,
            r.total_curve,
            1e-15,
            "total_curve",
        );
        close(
            round_tripped.total_sector,
            r.total_sector,
            1e-15,
            "total_sector",
        );
        close(
            round_tripped.total_selection,
            r.total_selection,
            1e-15,
            "total_selection",
        );
        assert_eq!(round_tripped.curve_effects.len(), r.curve_effects.len());
        assert_eq!(round_tripped.sector_effects.len(), r.sector_effects.len());
        assert_eq!(
            round_tripped.selection_effects.len(),
            r.selection_effects.len()
        );
        assert_eq!(round_tripped.curve_effects[0].cell, r.curve_effects[0].cell);
    }

    #[test]
    fn grid_position_serde_denies_unknown_fields() {
        let json = r#"{
            "cell": "0.0-3.0", "sector": "GOVT", "weight": 0.5, "total_return": 0.01
        }"#;
        let parsed: GridPosition = serde_json::from_str(json).expect("stable names parse");
        assert_eq!(parsed.cell, "0.0-3.0");

        let bad = r#"{
            "cell": "0.0-3.0", "sector": "GOVT", "weight": 0.5, "total_return": 0.01,
            "surprise": 1.0
        }"#;
        assert!(
            serde_json::from_str::<GridPosition>(bad).is_err(),
            "unknown field must be rejected"
        );
    }

    #[test]
    fn grid_attribution_rejects_non_finite_inputs() {
        let mut portfolio = vec![pos("0.0-3.0", "GOVT", 1.0, 0.01)];
        portfolio[0].total_return = f64::NAN;
        let benchmark = vec![pos("0.0-3.0", "GOVT", 1.0, 0.008)];
        let err = grid_attribution(&portfolio, &benchmark).expect_err("NaN must be rejected");
        assert!(err.to_string().contains("finite"), "{err}");
    }
}
