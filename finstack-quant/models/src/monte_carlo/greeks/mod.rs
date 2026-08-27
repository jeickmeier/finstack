//! Monte Carlo sensitivity estimators.
//!
//! This module groups the three Greek estimators used in the crate:
//! pathwise differentiation, likelihood-ratio / score-function methods, and
//! finite differences with common random numbers.
//!
//! Start with [`pathwise`] for smooth payoffs, use [`lrm`] for discontinuous
//! payoffs such as digitals or barriers (mind the per-estimator payoff
//! contracts documented on each function), and use [`finite_diff`] when you
//! need a general bump-and-reprice fallback. Host bindings for GBM European
//! finite-difference delta/gamma must call [`gbm_european`] rather than
//! assembling the engine themselves.

pub mod finite_diff;
pub mod gbm_european;
pub mod lrm;
pub mod pathwise;
