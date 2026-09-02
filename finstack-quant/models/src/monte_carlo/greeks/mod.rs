//! Monte Carlo sensitivity estimators.
//!
//! This module groups the two Greek estimators used in the crate:
//! likelihood-ratio / score-function methods and finite differences with
//! common random numbers.
//!
//! Use [`lrm`] for discontinuous payoffs such as digitals or barriers (mind
//! the per-estimator payoff contracts documented on each function), and use
//! [`finite_diff`] when you need a general bump-and-reprice fallback. Host bindings for GBM European
//! finite-difference delta/gamma must call [`gbm_european`] rather than
//! assembling the engine themselves.

pub mod finite_diff;
pub mod gbm_european;
pub mod lrm;
