//! Duration cell tables giving a base-return curve for duration-matched
//! credit excess return attribution.
//!
//! There are two ways to build a [`DurationCellTable`]:
//!
//! 1. [`cell_returns_from_reference`] implements the bucketing method of
//!    Dynkin, Hyman & Vankudre (1998), Appendix B, "Attribution of Portfolio
//!    Performance Relative to an Index": reference instruments (typically
//!    on-the-run Treasuries) are bucketed into fixed-width duration cells; a
//!    cell's base return is the **simple average** of the total returns of
//!    every reference instrument that falls in it ("calculate the average
//!    return of all Treasuries in that cell"). Cells with no reference
//!    instrument are filled by linear interpolation between the nearest
//!    observed neighbours (interior gaps) or flat extrapolation from the
//!    nearest observed cell (leading/trailing gaps — matching Lehman's
//!    convention of extending the longest observed cell, e.g. `12.5+ =
//!    1.16%`).
//! 2. [`cell_returns_from_curves`] derives each cell's base return directly
//!    from two curve snapshots (start- and end-of-period discount curves)
//!    instead of averaging discrete reference returns: a hypothetical
//!    zero-coupon position bought at the cell midpoint and revalued off the
//!    end curve. This makes the base curve a caller choice (Treasury or
//!    swap/OIS) rather than a hardcoded assumption, and every cell is
//!    observed since a curve has a value at every queried point.
//!
//! Either table is the foundation for duration-matched credit excess
//! returns: a credit position's excess return over its duration-matched
//! Treasury (or swap) cell isolates the credit-specific component of
//! performance from the general level/shape move of the base curve.
//!
//! # References
//!
//! * Dynkin, L., Hyman, J., & Vankudre, P. (1998). "Attribution of Portfolio
//!   Performance Relative to an Index." Lehman Brothers Fixed Income
//!   Research, March 1998, Appendix B.
//!   `docs/REFERENCES.md#dynkin-hyman-vankudre-1998`

use crate::error::{Error, Result};
use finstack_quant_core::market_data::term_structures::DiscountCurve;
use finstack_quant_core::math::summation::NeumaierAccumulator;
use serde::{Deserialize, Serialize};

/// Absolute tolerance for the position-weight-sums-to-one check in
/// [`excess_returns`], matching [`crate::fi_attribution`]'s convention.
const WEIGHT_TOLERANCE: f64 = 1e-6;

/// Configuration for [`cell_returns_from_reference`].
///
/// Deliberately a single field with no `Default` impl: an unspecified cell
/// width would silently pick an arbitrary granularity for the duration grid,
/// so callers must state it explicitly.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellConfig {
    /// Width of each duration cell, in years. Must be finite and positive.
    pub width: f64,
}

/// One reference (e.g. Treasury) instrument's duration and period total
/// return, used as an input to [`cell_returns_from_reference`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceReturn {
    /// Duration in years at period start. Must be finite and non-negative.
    pub duration: f64,
    /// Realized total return for the period (decimal, e.g. `0.01` = 1%).
    /// Must be finite.
    pub total_return: f64,
}

/// One duration cell's base return, observed or filled.
///
/// This type is *input-reachable*: later stages of duration-matched excess
/// return attribution consume [`DurationCellTable`] values, and the
/// Python/WASM bindings deserialize them from JSON. It therefore denies
/// unknown fields like the other inbound types in this crate, so a
/// misspelled or stale key fails closed instead of being silently dropped.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellReturn {
    /// Human-readable cell label, e.g. `"5.0-5.5"` (see
    /// [`duration_cell_label`]).
    pub label: String,
    /// Cell lower bound (inclusive), in years.
    pub lower: f64,
    /// Cell upper bound (exclusive), in years.
    ///
    /// One exception: the *top* cell (the cell containing the largest
    /// reference duration) folds a reference instrument whose duration is
    /// numerically equal to `upper` into this cell instead of raising `upper`
    /// to start a new, empty cell. This only happens for the single top cell
    /// of the table, and only when the largest reference duration is an
    /// exact multiple of the configured cell width (e.g. a duration of
    /// `2.0` with `width = 0.5` is folded into the `1.5-2.0` cell rather
    /// than opening an empty `2.0-2.5` cell). Every other cell boundary is
    /// strictly half-open.
    pub upper: f64,
    /// Cell base return: simple average of member reference returns, or
    /// interpolated/extrapolated when the cell has no members.
    pub base_return: f64,
    /// `true` if at least one reference instrument fell in this cell,
    /// `false` if the value was filled by interpolation or flat
    /// extrapolation.
    pub observed: bool,
}

/// A full duration-cell base-return curve built from a reference universe.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurationCellTable {
    /// Label identifying the reference universe this curve was built from
    /// (e.g. `"UST"`, `"SWAP"`).
    pub base_label: String,
    /// Cells in ascending duration order, spanning
    /// `[0, ceil(max_duration / width) * width)`.
    pub cells: Vec<CellReturn>,
}

/// Format a cell's bounds as a `"{lower:.1}-{upper:.1}"` label.
///
/// # Arguments
///
/// * `lower` - Cell lower bound, in years.
/// * `upper` - Cell upper bound, in years.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::excess_return::duration_cell_label;
///
/// assert_eq!(duration_cell_label(5.0, 5.5), "5.0-5.5");
/// ```
pub fn duration_cell_label(lower: f64, upper: f64) -> String {
    format!("{lower:.1}-{upper:.1}")
}

/// Validate a [`CellConfig::width`], shared by [`cell_returns_from_reference`]
/// and [`cell_returns_from_curves`].
fn validate_width(width: f64) -> Result<()> {
    if !width.is_finite() || width <= 0.0 {
        return Err(Error::invalid_input(format!(
            "CellConfig.width must be finite and positive (got {width})"
        )));
    }
    Ok(())
}

/// Validate one reference instrument's fields.
fn validate_reference(r: &ReferenceReturn, index: usize) -> Result<()> {
    if !r.duration.is_finite() || r.duration < 0.0 {
        return Err(Error::invalid_input(format!(
            "reference[{index}].duration must be finite and non-negative (got {})",
            r.duration
        )));
    }
    if !r.total_return.is_finite() {
        return Err(Error::invalid_input(format!(
            "reference[{index}].total_return must be finite (got {})",
            r.total_return
        )));
    }
    Ok(())
}

/// Build a duration-cell base-return table from a reference universe.
///
/// Buckets `reference` into cells of `config.width` years spanning
/// `[0, ceil(max_duration / width) * width)`, where `max_duration` is the
/// largest observed reference duration (there is no separate range
/// parameter in this stage — callers who need a wider grid than the data
/// supports should pad `reference` accordingly). Each cell's base return is
/// the simple (unweighted) average of the total returns of every reference
/// instrument whose duration falls in that cell. Empty interior cells are
/// filled by linear interpolation (in cell index) between the nearest
/// observed neighbours; empty leading/trailing cells are flat-extrapolated
/// from the nearest observed cell. See the module docs for the source
/// method (Dynkin, Hyman & Vankudre 1998, Appendix B).
///
/// Open-ended top cells (e.g. Lehman's `12.5+`) are not supported: the grid
/// always has a fixed upper bound derived from the data.
///
/// The top cell (the one containing the largest reference duration) is
/// therefore always [`CellReturn::observed`]; when that largest duration is
/// an exact multiple of `config.width`, the top cell folds it in rather than
/// opening a new empty cell above it — see the note on [`CellReturn::upper`].
///
/// # Arguments
///
/// * `reference` - Reference universe returns; must be non-empty.
/// * `base_label` - Label for the resulting curve (e.g. `"UST"`).
/// * `config` - Cell width configuration.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if `reference` is empty, `config.width`
/// is not finite and positive, any reference entry has a non-finite
/// `total_return` or a non-finite/negative `duration`, or `config.width`
/// produces two numerically distinct cells that format to the same
/// `"{lower:.1}-{upper:.1}"` label (e.g. `width = 0.03`) — labels must stay
/// unique because downstream lookups key cells by label.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::excess_return::{
///     cell_returns_from_reference, CellConfig, ReferenceReturn,
/// };
///
/// let reference = vec![
///     ReferenceReturn { duration: 0.25, total_return: 0.01 },
///     ReferenceReturn { duration: 2.25, total_return: 0.05 },
/// ];
/// let table = cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 })?;
/// assert_eq!(table.cells.len(), 5);
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
pub fn cell_returns_from_reference(
    reference: &[ReferenceReturn],
    base_label: &str,
    config: &CellConfig,
) -> Result<DurationCellTable> {
    if reference.is_empty() {
        return Err(Error::invalid_input(
            "cell_returns_from_reference requires a non-empty reference universe",
        ));
    }
    validate_width(config.width)?;
    for (index, r) in reference.iter().enumerate() {
        validate_reference(r, index)?;
    }

    let width = config.width;
    let max_duration = reference
        .iter()
        .map(|r| r.duration)
        .fold(f64::MIN, f64::max);
    let num_cells = ((max_duration / width).ceil() as usize).max(1);

    // Accumulate simple sums and counts per cell (index = floor(duration / width)).
    let mut sums = vec![0.0_f64; num_cells];
    let mut counts = vec![0_u32; num_cells];
    for r in reference {
        let mut idx = (r.duration / width) as usize;
        if idx >= num_cells {
            idx = num_cells - 1; // guards the exact-max-duration boundary case
        }
        sums[idx] += r.total_return;
        counts[idx] += 1;
    }

    let mut base_return = vec![0.0_f64; num_cells];
    let mut observed = vec![false; num_cells];
    for i in 0..num_cells {
        if counts[i] > 0 {
            base_return[i] = sums[i] / f64::from(counts[i]);
            observed[i] = true;
        }
    }

    fill_gaps(&mut base_return, &observed);

    let cells: Vec<CellReturn> = (0..num_cells)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let lower = i as f64 * width;
            let upper = lower + width;
            CellReturn {
                label: duration_cell_label(lower, upper),
                lower,
                upper,
                base_return: base_return[i],
                observed: observed[i],
            }
        })
        .collect();

    check_label_collisions(&cells, width)?;

    Ok(DurationCellTable {
        base_label: base_label.to_string(),
        cells,
    })
}

/// Build a duration-cell base-return table from two curve snapshots (start
/// and end of the holding period).
///
/// This is the second construction path for a [`DurationCellTable`]: instead
/// of averaging returns from a reference universe (see
/// [`cell_returns_from_reference`]), each cell's base return is the
/// holding-period total return of a hypothetical zero-coupon position bought
/// at the start of the period with maturity equal to the cell midpoint `d`,
/// and revalued off the `end` curve after `horizon_years` have elapsed:
///
/// ```text
/// R_cell(d) = df_end(d - Δt) / df_start(d) - 1,   Δt = horizon_years
/// ```
///
/// The same formula works identically for a Treasury discount curve or a
/// swap/OIS discount curve: the caller supplies whichever curves it wants and
/// stamps the choice into `base_label` (e.g. `"UST"`, `"USD-SOFR"`) purely for
/// policy visibility on [`DurationCellTable::base_label`] — it never affects
/// the arithmetic. Every cell in the resulting table is
/// [`CellReturn::observed`], because a curve supplies a discount factor at
/// every queried midpoint; there are no gaps to interpolate or extrapolate
/// (contrast with [`cell_returns_from_reference`], whose cells can be
/// empty).
///
/// Cells are indexed by the zero's remaining maturity at period start, which
/// is used as an approximation to Macaulay duration; this is exact for a
/// zero-coupon position (its only cash flow is the redemption, so duration
/// equals maturity) but is an approximation for coupon-bearing instruments in
/// later stages that reuse this table.
///
/// # Arguments
///
/// * `start` - Discount curve observed at the start of the holding period.
/// * `end` - Discount curve observed at the end of the holding period
///   (`horizon_years` later).
/// * `horizon_years` - Length of the holding period, in years. Must be finite
///   and positive.
/// * `max_duration` - Upper bound of the duration grid, in years. Must be
///   finite and strictly greater than `horizon_years`, so that at least one
///   cell's midpoint matures after the period ends.
/// * `base_label` - Label identifying the base curve (e.g. `"UST"`,
///   `"USD-SOFR"`), stamped into the result for policy visibility.
/// * `config` - Cell width configuration.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if `config.width` is not finite and
/// positive, `horizon_years` is not finite and positive, `max_duration` is
/// not finite or does not exceed `horizon_years`, any cell's midpoint `d`
/// does not exceed `horizon_years` (the hypothetical zero would mature inside
/// the holding period — the error names the offending cell), either curve
/// produces a non-finite or non-positive discount factor at a queried time,
/// or `config.width` produces colliding cell labels (see
/// [`cell_returns_from_reference`]'s error documentation).
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::excess_return::{cell_returns_from_curves, CellConfig};
/// use finstack_quant_core::market_data::term_structures::DiscountCurve;
/// use finstack_quant_core::dates::Date;
/// use time::Month;
///
/// let base = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
/// let start = DiscountCurve::builder("UST")
///     .base_date(base)
///     .knots([(0.0, 1.0), (0.5, 0.980_198_67), (1.5, 0.941_764_53)])
///     .build()?;
/// let end = DiscountCurve::builder("UST")
///     .base_date(base)
///     .knots([(0.0, 1.0), (0.25, 0.990_049_83), (1.25, 0.951_229_42)])
///     .build()?;
///
/// let table = cell_returns_from_curves(
///     &start, &end, 0.25, 2.0, "UST", &CellConfig { width: 1.0 },
/// )?;
/// assert_eq!(table.cells.len(), 2);
/// assert!(table.cells.iter().all(|c| c.observed));
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
///
/// # References
///
/// * Dynkin, L., Hyman, J., & Vankudre, P. (1998). "Attribution of Portfolio
///   Performance Relative to an Index." Lehman Brothers Fixed Income
///   Research, March 1998, Appendix B.
///   `docs/REFERENCES.md#dynkin-hyman-vankudre-1998`
pub fn cell_returns_from_curves(
    start: &DiscountCurve,
    end: &DiscountCurve,
    horizon_years: f64,
    max_duration: f64,
    base_label: &str,
    config: &CellConfig,
) -> Result<DurationCellTable> {
    validate_width(config.width)?;
    if !horizon_years.is_finite() || horizon_years <= 0.0 {
        return Err(Error::invalid_input(format!(
            "horizon_years must be finite and positive (got {horizon_years})"
        )));
    }
    if !max_duration.is_finite() || max_duration <= horizon_years {
        return Err(Error::invalid_input(format!(
            "max_duration must be finite and strictly greater than horizon_years \
             ({horizon_years}) (got {max_duration})"
        )));
    }

    let width = config.width;
    let num_cells = ((max_duration / width).ceil() as usize).max(1);

    let mut cells = Vec::with_capacity(num_cells);
    for i in 0..num_cells {
        #[allow(clippy::cast_precision_loss)]
        let lower = i as f64 * width;
        let upper = lower + width;
        let mid = lower + width / 2.0;
        let label = duration_cell_label(lower, upper);

        if mid <= horizon_years {
            return Err(Error::invalid_input(format!(
                "duration cell \"{label}\" (midpoint {mid}) does not exceed horizon_years \
                 ({horizon_years}): a zero-coupon position maturing at {mid} years would \
                 mature inside the holding period"
            )));
        }

        let start_t = mid;
        let end_t = mid - horizon_years;
        let df_start = start.df(start_t);
        let df_end = end.df(end_t);
        if !df_start.is_finite() || df_start <= 0.0 {
            return Err(Error::invalid_input(format!(
                "start curve '{}' produced a non-positive/non-finite discount factor \
                 ({df_start}) at t={start_t} for duration cell \"{label}\"",
                start.id()
            )));
        }
        if !df_end.is_finite() || df_end <= 0.0 {
            return Err(Error::invalid_input(format!(
                "end curve '{}' produced a non-positive/non-finite discount factor \
                 ({df_end}) at t={end_t} for duration cell \"{label}\"",
                end.id()
            )));
        }

        cells.push(CellReturn {
            label,
            lower,
            upper,
            base_return: df_end / df_start - 1.0,
            observed: true,
        });
    }

    check_label_collisions(&cells, width)?;

    Ok(DurationCellTable {
        base_label: base_label.to_string(),
        cells,
    })
}

/// Reject a `config.width` that makes two numerically distinct cells format
/// to the same `"{lower:.1}-{upper:.1}"` label.
///
/// Downstream stages (duration-matched excess return, grid attribution) look
/// up a position's base return by cell *label*, so a collision would
/// silently map two different duration buckets to whichever cell happened to
/// be inserted last, mis-pricing the excess return for one of them. Widths
/// that are multiples of `0.05` (and in particular of `0.1`) never collide;
/// finer widths such as `0.03` can, because `"{:.1}"` throws away
/// distinctions below one tenth of a year.
fn check_label_collisions(cells: &[CellReturn], width: f64) -> Result<()> {
    let mut seen: std::collections::HashSet<&str> =
        std::collections::HashSet::with_capacity(cells.len());
    for cell in cells {
        if !seen.insert(cell.label.as_str()) {
            return Err(Error::invalid_input(format!(
                "CellConfig.width {width} produces ambiguous duration cell labels at \
                 one-decimal precision: cell [{:.6}, {:.6}) collides with an earlier, \
                 numerically distinct cell on label \"{}\". Choose a width (e.g. a multiple \
                 of 0.1) whose \"{{lower:.1}}-{{upper:.1}}\" labels stay unique.",
                cell.lower, cell.upper, cell.label
            )));
        }
    }
    Ok(())
}

/// Fill unobserved entries of `base_return` in place: interior gaps by
/// linear interpolation (by cell index) between the nearest observed
/// neighbours, leading/trailing gaps by flat extrapolation from the nearest
/// observed cell.
///
/// Precondition (guaranteed by the only caller): at least one entry of
/// `observed` is `true`, since every reference instrument is bucketed into
/// some cell.
fn fill_gaps(base_return: &mut [f64], observed: &[bool]) {
    let n = base_return.len();

    // Leading flat extrapolation: fill cells before the first observed one.
    if let Some(first) = observed.iter().position(|&o| o) {
        let first_value = base_return[first];
        for value in &mut base_return[..first] {
            *value = first_value;
        }
    }

    // Trailing flat extrapolation: fill cells after the last observed one.
    if let Some(last) = observed.iter().rposition(|&o| o) {
        let last_value = base_return[last];
        for value in &mut base_return[last + 1..n] {
            *value = last_value;
        }
    }

    // Interior linear interpolation (by cell index) between consecutive
    // observed cells.
    let observed_indices: Vec<usize> = observed
        .iter()
        .enumerate()
        .filter_map(|(i, &o)| o.then_some(i))
        .collect();
    for pair in observed_indices.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        let gap = hi - lo;
        if gap <= 1 {
            continue;
        }
        let lo_value = base_return[lo];
        let hi_value = base_return[hi];
        #[allow(clippy::cast_precision_loss)]
        let gap_f = gap as f64;
        for step in 1..gap {
            #[allow(clippy::cast_precision_loss)]
            let t = step as f64 / gap_f;
            base_return[lo + step] = lo_value + t * (hi_value - lo_value);
        }
    }
}

/// One position's beginning-of-period duration, weight and realized total
/// return, the input to [`excess_returns`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcessReturnPosition {
    /// Position identifier, used to name the position in any validation
    /// error and echoed onto the matching [`PositionExcess::id`].
    pub id: String,
    /// Portfolio weight (decimal, e.g. `0.2` = 20%). Across all positions
    /// passed to [`excess_returns`] these must sum to `1.0` within
    /// [`WEIGHT_TOLERANCE`]. Must be finite.
    pub weight: f64,
    /// Beginning-of-period duration in years, used to look up the
    /// duration-matched cell in the [`DurationCellTable`]. Must be finite,
    /// non-negative, and fall within the table's covered range
    /// `[0, cells.last().upper]`.
    pub duration: f64,
    /// Realized period total return (decimal, e.g. `0.0225` = 2.25%). Must
    /// be finite.
    pub total_return: f64,
}

/// One position's duration-matched credit excess return.
///
/// This type is *input-reachable*: the Python/WASM bindings deserialize
/// [`ExcessReturnResult`] (which embeds a `Vec` of these) from JSON, so it
/// denies unknown fields like the other inbound types in this module.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionExcess {
    /// Position identifier (mirrors [`ExcessReturnPosition::id`]).
    pub id: String,
    /// Label of the duration cell the position was matched to (mirrors
    /// [`CellReturn::label`]).
    pub cell: String,
    /// The matched cell's base return.
    pub base_return: f64,
    /// `total_return − base_return` for this position.
    pub excess_return: f64,
}

/// Per-position and portfolio-level duration-matched credit excess return
/// result, produced by [`excess_returns`].
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExcessReturnResult {
    /// Label of the base curve the excess returns were measured against
    /// (mirrors [`DurationCellTable::base_label`]).
    pub base_label: String,
    /// Per-position results, in input order.
    pub positions: Vec<PositionExcess>,
    /// Weight-weighted portfolio total return `Σ w_i · total_return_i`.
    pub portfolio_total_return: f64,
    /// Weight-weighted portfolio base return `Σ w_i · base_return_i`.
    pub portfolio_base_return: f64,
    /// Weight-weighted portfolio excess return `Σ w_i · excess_return_i`,
    /// computed independently from `portfolio_total_return` and
    /// `portfolio_base_return` (not as their difference). The two are
    /// algebraically identical; see [`excess_returns`]'s tests for the
    /// reconciling identity.
    pub portfolio_excess_return: f64,
}

/// Validate one [`ExcessReturnPosition`]'s numeric fields.
fn validate_position(p: &ExcessReturnPosition) -> Result<()> {
    if !p.weight.is_finite() {
        return Err(Error::invalid_input(format!(
            "position '{}' weight must be finite (got {})",
            p.id, p.weight
        )));
    }
    if !p.duration.is_finite() || p.duration < 0.0 {
        return Err(Error::invalid_input(format!(
            "position '{}' duration must be finite and non-negative (got {})",
            p.id, p.duration
        )));
    }
    if !p.total_return.is_finite() {
        return Err(Error::invalid_input(format!(
            "position '{}' total_return must be finite (got {})",
            p.id, p.total_return
        )));
    }
    Ok(())
}

/// Find the [`CellReturn`] whose duration range contains `duration`.
///
/// Cells are half-open `[lower, upper)`, except the table's last cell, which
/// is closed `[lower, upper]` — mirroring the inclusive-top-cell convention
/// documented on [`CellReturn::upper`], so a position sitting exactly at the
/// table's outer edge is not spuriously rejected.
fn find_cell<'a>(
    duration: f64,
    table: &'a DurationCellTable,
    position_id: &str,
) -> Result<&'a CellReturn> {
    let last_index = table.cells.len().saturating_sub(1);
    for (index, cell) in table.cells.iter().enumerate() {
        let in_range = if index == last_index {
            duration >= cell.lower && duration <= cell.upper
        } else {
            duration >= cell.lower && duration < cell.upper
        };
        if in_range {
            return Ok(cell);
        }
    }
    let (lower, upper) = (
        table.cells.first().map_or(0.0, |c| c.lower),
        table.cells.last().map_or(0.0, |c| c.upper),
    );
    Err(Error::invalid_input(format!(
        "position '{position_id}' duration {duration} falls outside the duration cell \
         table's covered range [{lower:.6}, {upper:.6}]"
    )))
}

/// Compute duration-matched credit excess returns for a set of positions
/// against a [`DurationCellTable`] base curve.
///
/// Implements the position-level application of Dynkin, Hyman & Vankudre
/// (1998), Appendix B: each position's beginning-of-period `duration` is
/// mapped to its duration cell in `table`, and the position's excess return
/// is `total_return − cell.base_return` — the credit-specific component of
/// performance, isolated from the general level/shape move of the base
/// curve. Portfolio-level totals are the weight-weighted sums of the
/// per-position total, base and excess returns (each accumulated
/// independently with [`NeumaierAccumulator`] for numerical stability), so
/// the identity `portfolio_excess_return == portfolio_total_return −
/// portfolio_base_return` is a check on the arithmetic rather than a
/// shortcut taken by it.
///
/// # Arguments
///
/// * `positions` - Positions to attribute; weights must sum to `1.0` within
///   [`WEIGHT_TOLERANCE`].
/// * `table` - Duration-cell base-return curve to match positions against
///   (see [`cell_returns_from_reference`] or [`cell_returns_from_curves`]).
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if `table` has no cells, any position has
/// a non-finite `weight`/`total_return` or a non-finite/negative `duration`,
/// any position's `duration` falls outside `table`'s covered range (the
/// error names the position by `id`), or the position weights do not sum to
/// `1.0` within [`WEIGHT_TOLERANCE`].
///
/// # Examples
///
/// ```rust
/// use finstack_quant_portfolio::excess_return::{
///     cell_returns_from_reference, excess_returns, CellConfig, ExcessReturnPosition,
///     ReferenceReturn,
/// };
///
/// let reference = vec![
///     ReferenceReturn { duration: 0.25, total_return: 0.01 },
///     ReferenceReturn { duration: 2.25, total_return: 0.05 },
/// ];
/// let table = cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 })?;
/// let positions = vec![ExcessReturnPosition {
///     id: "Bond1".into(),
///     weight: 1.0,
///     duration: 0.4,
///     total_return: 0.02,
/// }];
/// let result = excess_returns(&positions, &table)?;
/// assert!((result.positions[0].excess_return - 0.01).abs() < 1e-12);
/// # Ok::<(), finstack_quant_portfolio::Error>(())
/// ```
///
/// # References
///
/// * Dynkin, L., Hyman, J., & Vankudre, P. (1998). "Attribution of Portfolio
///   Performance Relative to an Index." Lehman Brothers Fixed Income
///   Research, March 1998, Appendix B.
///   `docs/REFERENCES.md#dynkin-hyman-vankudre-1998`
pub fn excess_returns(
    positions: &[ExcessReturnPosition],
    table: &DurationCellTable,
) -> Result<ExcessReturnResult> {
    if table.cells.is_empty() {
        return Err(Error::invalid_input(
            "excess_returns requires a DurationCellTable with at least one cell",
        ));
    }

    let mut weight_sum = NeumaierAccumulator::new();
    let mut total_return_sum = NeumaierAccumulator::new();
    let mut base_return_sum = NeumaierAccumulator::new();
    let mut excess_return_sum = NeumaierAccumulator::new();
    let mut position_results = Vec::with_capacity(positions.len());

    for p in positions {
        validate_position(p)?;
        let cell = find_cell(p.duration, table, &p.id)?;
        let excess = p.total_return - cell.base_return;

        weight_sum.add(p.weight);
        total_return_sum.add(p.weight * p.total_return);
        base_return_sum.add(p.weight * cell.base_return);
        excess_return_sum.add(p.weight * excess);

        position_results.push(PositionExcess {
            id: p.id.clone(),
            cell: cell.label.clone(),
            base_return: cell.base_return,
            excess_return: excess,
        });
    }

    let total_weight = weight_sum.total();
    if (total_weight - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(Error::invalid_input(format!(
            "position weights must sum to 1.0 (got {total_weight})"
        )));
    }

    Ok(ExcessReturnResult {
        base_label: table.base_label.clone(),
        positions: position_results,
        portfolio_total_return: total_return_sum.total(),
        portfolio_base_return: base_return_sum.total(),
        portfolio_excess_return: excess_return_sum.total(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    /// Builds a [`DiscountCurve`] from exact `(t, df)` knots for curve-snapshot
    /// tests. The brief's helper omitted `.base_date(..)`; `DiscountCurveBuilder::build`
    /// requires a base date (`base: Option<Date>` starts `None` and `build()` errors
    /// on `None`), so it is added here. `df(t)` reads knot times as year fractions
    /// directly and never consults the base date, so the choice of date is
    /// otherwise inert for these tests.
    fn curve(id: &'static str, knots: &[(f64, f64)]) -> DiscountCurve {
        DiscountCurve::builder(id)
            .base_date(date!(2024 - 01 - 01))
            .knots(knots.iter().copied())
            .build()
            .expect("curve")
    }

    #[test]
    fn flat_curve_cell_returns_equal_pure_carry() {
        // 4% flat, Δt = 0.25, width 1.0, max 2.0 => mids 0.5 and 1.5.
        // R = e^{0.04·0.25} − 1 = 0.01005017 for every cell (carry only, no roll differential).
        let knots = [
            (0.0, 1.0),
            (0.25, 0.99004983),
            (0.5, 0.98019867),
            (1.25, 0.95122942),
            (1.5, 0.94176453),
        ];
        let start = curve("UST", &knots);
        let end = curve("UST", &knots);
        let table =
            cell_returns_from_curves(&start, &end, 0.25, 2.0, "UST", &CellConfig { width: 1.0 })
                .unwrap();
        assert_eq!(table.cells.len(), 2);
        for cell in &table.cells {
            assert!(
                (cell.base_return - 0.01005017).abs() < 1e-6,
                "{}",
                cell.base_return
            );
            assert!(cell.observed);
        }
    }

    #[test]
    fn rising_rates_hurt_the_longer_cell_more() {
        // Start 4% flat, end 5% flat: R(0.5) = e^{−0.0125+0.02} − 1, R(1.5) = e^{−0.0625+0.06} − 1.
        //
        // Two corrections to the brief here, both verified with Python
        // `decimal.Decimal.exp` at 50-digit precision:
        //
        // 1. The brief's second end-curve knot literal `(0.25, 0.98757805)` is
        //    wrong: e^{-0.05*0.25} = e^{-0.0125} = 0.9875778004938814..., which
        //    rounds to `0.98757780`, not `0.98757805` — a ~2.5e-7 error, not
        //    just an 8th-decimal rounding artifact. Using the wrong knot as
        //    written makes `df_end(0.25)` (and hence `R(0.5)`) numerically
        //    wrong regardless of interpolation method, since the curve passes
        //    through its own knots exactly. Fixed to `0.98757780` below (the
        //    other three knots in this test were independently verified
        //    correct to 8 decimals).
        // 2. With the corrected knot, R(0.5) = 0.0075281983396..., not the
        //    brief's `0.00752821` (off by ~1.46e-8, outside the brief's own
        //    `1e-8` tolerance). R(1.5) = -0.0024968776025398..., which does
        //    match the brief's `-0.00249688` within `1e-8`.
        let start = curve("UST", &[(0.0, 1.0), (0.5, 0.98019867), (1.5, 0.94176453)]);
        let end = curve("UST", &[(0.0, 1.0), (0.25, 0.98757780), (1.25, 0.93941306)]);
        let table =
            cell_returns_from_curves(&start, &end, 0.25, 2.0, "UST", &CellConfig { width: 1.0 })
                .unwrap();
        assert!((table.cells[0].base_return - 0.0075281983).abs() < 1e-8);
        assert!((table.cells[1].base_return - (-0.00249688)).abs() < 1e-8);
    }

    #[test]
    fn swap_curve_base_is_supported_via_label() {
        let knots = [
            (0.0, 1.0),
            (0.25, 0.99004983),
            (0.5, 0.98019867),
            (1.25, 0.95122942),
            (1.5, 0.94176453),
        ];
        let table = cell_returns_from_curves(
            &curve("USD-SOFR", &knots),
            &curve("USD-SOFR", &knots),
            0.25,
            2.0,
            "USD-SOFR",
            &CellConfig { width: 1.0 },
        )
        .unwrap();
        assert_eq!(table.base_label, "USD-SOFR");
    }

    #[test]
    fn cell_maturing_inside_period_is_rejected() {
        let knots = [(0.0, 1.0), (0.25, 0.99004983), (0.5, 0.98019867)];
        // width 0.25 => first mid 0.125 < Δt 0.25 => error naming the cell
        let err = cell_returns_from_curves(
            &curve("UST", &knots),
            &curve("UST", &knots),
            0.25,
            0.5,
            "UST",
            &CellConfig { width: 0.25 },
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("0.0-0.2") || err.to_string().contains("matures"),
            "{err}"
        );
    }

    // NOTE on the Task 1 carried finding (trailing flat extrapolation through
    // a public API): `cell_returns_from_curves` does not close this gap.
    // Every cell it produces is `observed: true` by construction — a curve
    // supplies a discount factor at every queried midpoint, so there is
    // nothing to interpolate or extrapolate (see the module-level ambiguity
    // note and this task's report for the full reasoning). And
    // `cell_returns_from_reference` still cannot reach the trailing branch
    // either: `num_cells = ceil(max_duration / width)` is derived from the
    // *largest reference duration itself*, so the top cell always contains
    // that duration and is always observed — there is no parameter that lets
    // a caller extend the grid past the data, by design (see that function's
    // doc comment: "there is no separate range parameter in this stage").
    // The gap remains covered only by the direct `fill_gaps` unit test
    // (`fill_gaps_extrapolates_leading_and_trailing_flat` above); closing it
    // through a public API would require a future stage to add an explicit
    // upper-bound parameter to `cell_returns_from_reference` itself.

    #[test]
    fn cell_table_interpolates_interior_and_extrapolates_ends() {
        // Observed: cell 0.0-0.5 => 0.01, cell 2.0-2.5 => 0.05. Gaps 0.5-2.0 interpolate
        // linearly by cell index: 0.02, 0.03, 0.04. A trailing empty cell would be flat.
        let reference = vec![
            ReferenceReturn {
                duration: 0.25,
                total_return: 0.01,
            },
            ReferenceReturn {
                duration: 2.25,
                total_return: 0.05,
            },
        ];
        let table = cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 })
            .expect("valid reference");
        let returns: Vec<f64> = table.cells.iter().map(|c| c.base_return).collect();
        let observed: Vec<bool> = table.cells.iter().map(|c| c.observed).collect();
        assert_eq!(table.cells.len(), 5);
        assert_eq!(table.cells[1].label, "0.5-1.0");
        for (got, want) in returns.iter().zip([0.01, 0.02, 0.03, 0.04, 0.05]) {
            assert!((got - want).abs() < 1e-12, "{got} vs {want}");
        }
        assert_eq!(observed, [true, false, false, false, true]);
        assert_eq!(table.base_label, "UST");
    }

    /// Leading flat extrapolation through the public API: the smallest
    /// reference duration lands strictly above cell 0, so cells before it
    /// have no members and must flat-equal the first observed cell.
    ///
    /// Note there is no equivalent trailing case reachable through this
    /// function: `num_cells` is derived from the *largest* reference
    /// duration (`ceil(max_duration / width)`), so the top cell always
    /// contains that duration and is therefore always observed by
    /// construction. Trailing extrapolation is still real code, reachable
    /// once a future stage adds an explicit upper-bound parameter that can
    /// exceed the observed maximum; the
    /// `fill_gaps_extrapolates_leading_and_trailing_flat` test below
    /// exercises it directly against the internal `fill_gaps` helper.
    #[test]
    fn cell_table_extrapolates_leading_cells_flat() {
        let reference = vec![
            ReferenceReturn {
                duration: 1.25, // cell 2 of a 0.5-wide grid: [1.0, 1.5)
                total_return: 0.02,
            },
            ReferenceReturn {
                duration: 2.25, // top cell: [2.0, 2.5)
                total_return: 0.05,
            },
        ];
        let table = cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 })
            .expect("valid reference");
        assert_eq!(table.cells.len(), 5);
        // Cells 0 and 1 are leading gaps below the first observed cell (2).
        assert_eq!(
            table.cells[0].base_return, 0.02,
            "cell 0 must flat-extrapolate from the first observed cell"
        );
        assert!(!table.cells[0].observed);
        assert_eq!(
            table.cells[1].base_return, 0.02,
            "cell 1 must flat-extrapolate from the first observed cell"
        );
        assert!(!table.cells[1].observed);
        assert!(table.cells[2].observed);
    }

    /// Directly exercises both flat-extrapolation branches of the internal
    /// `fill_gaps` helper (leading *and* trailing), which
    /// `cell_table_extrapolates_leading_cells_flat` above cannot reach on the
    /// trailing side alone (see that test's doc comment for why). Mirrors
    /// the reviewer's "cells 2 and 4 of a 6-cell grid" construction: with
    /// observed cells at index 2 and 4, cells 0-1 must flat-equal cell 2's
    /// value and cell 5 must flat-equal cell 4's value.
    #[test]
    fn fill_gaps_extrapolates_leading_and_trailing_flat() {
        let mut base_return = vec![0.0, 0.0, 0.03, 0.0, 0.07, 0.0];
        let observed = [false, false, true, false, true, false];
        fill_gaps(&mut base_return, &observed);
        assert_eq!(
            base_return[0], 0.03,
            "leading cell 0 must flat-equal cell 2"
        );
        assert_eq!(
            base_return[1], 0.03,
            "leading cell 1 must flat-equal cell 2"
        );
        assert_eq!(base_return[2], 0.03, "observed cell 2 must be unchanged");
        assert_eq!(base_return[4], 0.07, "observed cell 4 must be unchanged");
        assert_eq!(
            base_return[5], 0.07,
            "trailing cell 5 must flat-equal cell 4"
        );
    }

    #[test]
    fn cell_table_averages_multiple_instruments_per_cell() {
        let reference = vec![
            ReferenceReturn {
                duration: 0.10,
                total_return: 0.010,
            },
            ReferenceReturn {
                duration: 0.40,
                total_return: 0.030,
            }, // same cell 0.0-0.5
        ];
        let table =
            cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 }).unwrap();
        assert!((table.cells[0].base_return - 0.020).abs() < 1e-15); // simple average
    }

    #[test]
    fn cell_table_rejects_bad_inputs() {
        let ok = vec![ReferenceReturn {
            duration: 1.0,
            total_return: 0.01,
        }];
        assert!(cell_returns_from_reference(&[], "UST", &CellConfig { width: 0.5 }).is_err());
        assert!(cell_returns_from_reference(&ok, "UST", &CellConfig { width: 0.0 }).is_err());
        let neg = vec![ReferenceReturn {
            duration: -0.5,
            total_return: 0.01,
        }];
        assert!(cell_returns_from_reference(&neg, "UST", &CellConfig { width: 0.5 }).is_err());
        let nan = vec![ReferenceReturn {
            duration: 1.0,
            total_return: f64::NAN,
        }];
        assert!(cell_returns_from_reference(&nan, "UST", &CellConfig { width: 0.5 }).is_err());
    }

    /// `width = 0.03` makes distinct cells (e.g. `[0.06, 0.09)` and
    /// `[0.09, 0.12)`) both format to the label `"0.1-0.1"` under
    /// `"{lower:.1}-{upper:.1}"`. Since downstream stages look positions up
    /// by label, this must fail closed rather than silently colliding.
    #[test]
    fn cell_table_rejects_colliding_width_labels() {
        let reference = vec![ReferenceReturn {
            duration: 0.1,
            total_return: 0.01,
        }];
        let err = cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.03 })
            .expect_err("width=0.03 must be rejected: labels collide at one-decimal precision");
        let message = err.to_string();
        assert!(message.contains("0.03"), "must name the width: {message}");
        assert!(
            message.contains("0.1-0.1"),
            "must name the colliding label: {message}"
        );
    }

    /// Widths that are multiples of `0.1` (the plan-mandated widths used by
    /// this task and Task 2) must keep passing the collision check.
    #[test]
    fn cell_table_accepts_legitimate_widths() {
        let reference = vec![
            ReferenceReturn {
                duration: 0.25,
                total_return: 0.01,
            },
            ReferenceReturn {
                duration: 2.25,
                total_return: 0.05,
            },
        ];
        assert!(cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 }).is_ok());
        assert!(
            cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.25 }).is_ok()
        );
    }

    /// Populated cells of Lehman Figure B-1 (May 1997 Treasury returns by
    /// duration), percent/100. Empty cells 7.5-9.0 and 12.5+ are deliberately
    /// omitted — interpolation/extrapolation is covered by Task 1's own tests.
    fn lehman_b1_table() -> DurationCellTable {
        let cells: &[(f64, f64)] = &[
            (0.25, 0.0054),
            (0.75, 0.0059),
            (1.25, 0.0066),
            (1.75, 0.0071),
            (2.25, 0.0073),
            (2.75, 0.0075),
            (3.25, 0.0078),
            (3.75, 0.0078),
            (4.25, 0.0080),
            (4.75, 0.0092),
            (5.25, 0.0097),
            (5.75, 0.0103),
            (6.25, 0.0111),
            (6.75, 0.0105),
            (7.25, 0.0105),
            (9.25, 0.0110),
            (9.75, 0.0111),
            (10.25, 0.0111),
            (10.75, 0.0115),
            (11.25, 0.0118),
            (11.75, 0.0121),
            (12.25, 0.0116),
        ];
        let reference: Vec<ReferenceReturn> = cells
            .iter()
            .map(|&(duration, total_return)| ReferenceReturn {
                duration,
                total_return,
            })
            .collect();
        cell_returns_from_reference(&reference, "UST", &CellConfig { width: 0.5 }).unwrap()
    }

    /// Reproduces Dynkin, Hyman & Vankudre (1998), Figure B-2 exactly: five
    /// real bonds, five published excess returns. Independently re-derived
    /// (see task report) with Python at full precision; the brief's values
    /// match the arithmetic exactly, so this asserts at `1e-12`.
    #[test]
    fn lehman_figure_b2_excess_returns_reproduce_exactly() {
        let table = lehman_b1_table();
        let positions = vec![
            ExcessReturnPosition {
                id: "Colombia".into(),
                weight: 0.2,
                duration: 5.16,
                total_return: 0.0225,
            },
            ExcessReturnPosition {
                id: "RiteAid".into(),
                weight: 0.2,
                duration: 6.71,
                total_return: 0.0172,
            },
            ExcessReturnPosition {
                id: "NewsAM".into(),
                weight: 0.2,
                duration: 9.58,
                total_return: 0.0182,
            },
            ExcessReturnPosition {
                id: "Delta".into(),
                weight: 0.2,
                duration: 9.81,
                total_return: 0.0102,
            },
            ExcessReturnPosition {
                id: "Quebec".into(),
                weight: 0.2,
                duration: 11.08,
                total_return: 0.0185,
            },
        ];
        let result = excess_returns(&positions, &table).unwrap();
        let expected = [0.0128, 0.0067, 0.0071, -0.0009, 0.0067]; // Figure B-2, exact
        for (got, want) in result.positions.iter().zip(expected) {
            assert!(
                (got.excess_return - want).abs() < 1e-12,
                "{}: {} vs {want}",
                got.id,
                got.excess_return
            );
        }
        assert_eq!(result.positions[0].cell, "5.0-5.5");
        assert!((result.portfolio_excess_return - 0.00648).abs() < 1e-12); // 0.2 * Sum(excess)
        assert!((result.portfolio_total_return - 0.01732).abs() < 1e-12);
        assert!((result.portfolio_base_return - 0.01084).abs() < 1e-12);
        // Identity: excess === total - base, computed independently.
        assert!(
            (result.portfolio_excess_return
                - (result.portfolio_total_return - result.portfolio_base_return))
                .abs()
                < 1e-15
        );
    }

    #[test]
    fn excess_returns_validation() {
        let table = lehman_b1_table();
        // duration beyond table
        let far = vec![ExcessReturnPosition {
            id: "X".into(),
            weight: 1.0,
            duration: 99.0,
            total_return: 0.01,
        }];
        let err = excess_returns(&far, &table).unwrap_err();
        assert!(err.to_string().contains('X'), "{err}");
        // weight tolerance pinned both directions
        let mk = |w: f64| {
            vec![ExcessReturnPosition {
                id: "A".into(),
                weight: w,
                duration: 1.0,
                total_return: 0.01,
            }]
        };
        assert!(excess_returns(&mk(1.0 + 5e-7), &table).is_ok());
        assert!(excess_returns(&mk(1.0 + 2e-6), &table).is_err());
    }

    /// Serialize -> deserialize round-trip for [`ExcessReturnResult`].
    ///
    /// Float fields are compared with a tight tolerance rather than
    /// `assert_eq!`: this workspace's `serde_json` dependency does not enable
    /// the `float_roundtrip` cargo feature, so JSON float parsing is
    /// documented as "best-effort precision," not bit-exact — independently
    /// confirmed here by round-tripping `0.0225 - 0.0097` (`0.01279999999999999888`)
    /// through `serde_json`, which comes back as `0.01280000000000000061`, one
    /// ULP off. That is a `serde_json` parser characteristic, not a defect in
    /// this module, so the test tolerates it instead of masking it with
    /// `float_roundtrip`.
    #[test]
    fn excess_return_result_serde_round_trip() {
        let table = lehman_b1_table();
        let positions = vec![ExcessReturnPosition {
            id: "Colombia".into(),
            weight: 1.0,
            duration: 5.16,
            total_return: 0.0225,
        }];
        let result = excess_returns(&positions, &table).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let round_tripped: ExcessReturnResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped.base_label, result.base_label);
        assert_eq!(round_tripped.positions.len(), result.positions.len());
        assert_eq!(round_tripped.positions[0].id, result.positions[0].id);
        assert_eq!(round_tripped.positions[0].cell, result.positions[0].cell);
        assert!(
            (round_tripped.positions[0].base_return - result.positions[0].base_return).abs()
                < 1e-12
        );
        assert!(
            (round_tripped.positions[0].excess_return - result.positions[0].excess_return).abs()
                < 1e-12
        );
        assert!(
            (round_tripped.portfolio_total_return - result.portfolio_total_return).abs() < 1e-12
        );
        assert!((round_tripped.portfolio_base_return - result.portfolio_base_return).abs() < 1e-12);
        assert!(
            (round_tripped.portfolio_excess_return - result.portfolio_excess_return).abs() < 1e-12
        );
    }

    #[test]
    fn excess_return_position_denies_unknown_fields() {
        let json =
            r#"{"id": "A", "weight": 1.0, "duration": 1.0, "total_return": 0.01, "extra": 1}"#;
        let result: std::result::Result<ExcessReturnPosition, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
