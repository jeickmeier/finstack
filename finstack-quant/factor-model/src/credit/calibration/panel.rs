use std::collections::BTreeMap;

use finstack_quant_core::dates::Date;
use finstack_quant_core::types::IssuerId;

use super::config::PanelSpace;
use crate::credit::hierarchy::{IssuerBetaMode, IssuerBetaOverride, IssuerBetaPolicy};

/// Step 1: classify an issuer as `IssuerBeta` or `BucketOnly`.
///
/// Under `Dynamic { min_history, .. }` the gate counts the observations the
/// regression will actually use in the configured panel space: raw `Some`
/// levels for [`PanelSpace::Levels`], consecutive `Some` pairs (usable return
/// observations) for [`PanelSpace::Returns`]. Counting raw levels under
/// `Returns` would overstate the usable history on gappy panels.
pub(super) fn classify_mode(
    policy: &IssuerBetaPolicy,
    issuer: &IssuerId,
    spreads: &BTreeMap<IssuerId, Vec<Option<f64>>>,
    space: &PanelSpace,
) -> IssuerBetaMode {
    match policy {
        IssuerBetaPolicy::GloballyOff => IssuerBetaMode::BucketOnly,
        IssuerBetaPolicy::Dynamic {
            min_history,
            overrides,
        } => match overrides.get(issuer) {
            Some(IssuerBetaOverride::ForceIssuerBeta) => IssuerBetaMode::IssuerBeta,
            Some(IssuerBetaOverride::ForceBucketOnly) => IssuerBetaMode::BucketOnly,
            Some(IssuerBetaOverride::Auto) | None => {
                let count = spreads
                    .get(issuer)
                    .map(|s| match space {
                        PanelSpace::Levels => s.iter().filter(|v| v.is_some()).count(),
                        PanelSpace::Returns => s
                            .windows(2)
                            .filter(|w| w[0].is_some() && w[1].is_some())
                            .count(),
                    })
                    .unwrap_or(0);
                if count >= *min_history {
                    IssuerBetaMode::IssuerBeta
                } else {
                    IssuerBetaMode::BucketOnly
                }
            }
        },
    }
}

/// Working panel after step 2 (returns or levels).
pub(super) struct WorkingPanel {
    /// Generic factor series in the chosen space, length = dates.len() - 1 (Returns)
    /// or dates.len() (Levels).
    pub(super) generic: Vec<f64>,
    /// Per-issuer aligned values (`None` for missing observations / missing pair).
    pub(super) issuers: BTreeMap<IssuerId, Vec<Option<f64>>>,
}

/// First-difference a sparse series: `d[t] = s[t+1] - s[t]` where both
/// observations exist, `None` otherwise. Length is `len - 1`.
pub(super) fn diff_sparse(series: &[Option<f64>]) -> Vec<Option<f64>> {
    series
        .windows(2)
        .map(|w| match (w[0], w[1]) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        })
        .collect()
}

pub(super) fn build_working_panel(
    space: &PanelSpace,
    dates: &[Date],
    spreads: &BTreeMap<IssuerId, Vec<Option<f64>>>,
    generic: &[f64],
) -> WorkingPanel {
    match space {
        PanelSpace::Levels => WorkingPanel {
            generic: generic.to_vec(),
            issuers: spreads.clone(),
        },
        PanelSpace::Returns => {
            let n = dates.len();
            let mut g = Vec::with_capacity(n.saturating_sub(1));
            for t in 1..n {
                g.push(generic[t] - generic[t - 1]);
            }
            let mut issuers: BTreeMap<IssuerId, Vec<Option<f64>>> = BTreeMap::new();
            for (issuer, series) in spreads {
                issuers.insert(issuer.clone(), diff_sparse(series));
            }
            WorkingPanel {
                generic: g,
                issuers,
            }
        }
    }
}
