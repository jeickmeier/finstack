//! Duration cell tables built from a reference (e.g. Treasury) universe.
//!
//! Implements the bucketing method of Dynkin, Hyman & Vankudre (1998),
//! Appendix B, "Attribution of Portfolio Performance Relative to an Index":
//! reference instruments (typically on-the-run Treasuries) are bucketed into
//! fixed-width duration cells; a cell's base return is the **simple
//! average** of the total returns of every reference instrument that falls
//! in it ("calculate the average return of all Treasuries in that cell").
//! Cells with no reference instrument are filled by linear interpolation
//! between the nearest observed neighbours (interior gaps) or flat
//! extrapolation from the nearest observed cell (leading/trailing gaps —
//! matching Lehman's convention of extending the longest observed cell, e.g.
//! `12.5+ = 1.16%`).
//!
//! This duration-cell base-return curve is the foundation for duration-matched
//! credit excess returns: a credit position's excess return over its
//! duration-matched Treasury (or swap) cell isolates the credit-specific
//! component of performance from the general level/shape move of the base
//! curve.
//!
//! # References
//!
//! * Dynkin, L., Hyman, J., & Vankudre, P. (1998). "Attribution of Portfolio
//!   Performance Relative to an Index." Lehman Brothers Fixed Income
//!   Research, March 1998, Appendix B.
//!   `docs/REFERENCES.md#dynkin-hyman-vankudre-1998`

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

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
/// # Arguments
///
/// * `reference` - Reference universe returns; must be non-empty.
/// * `base_label` - Label for the resulting curve (e.g. `"UST"`).
/// * `config` - Cell width configuration.
///
/// # Errors
///
/// Returns [`Error::InvalidInput`] if `reference` is empty, `config.width`
/// is not finite and positive, or any reference entry has a non-finite
/// `total_return` or a non-finite/negative `duration`.
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
    if !config.width.is_finite() || config.width <= 0.0 {
        return Err(Error::invalid_input(format!(
            "CellConfig.width must be finite and positive (got {})",
            config.width
        )));
    }
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

    let cells = (0..num_cells)
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

    Ok(DurationCellTable {
        base_label: base_label.to_string(),
        cells,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
