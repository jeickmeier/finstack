//! Core data structures for P&L attribution.
//!
//! This module provides types for decomposing multi-period P&L changes into
//! constituent factors: carry, curve shifts, credit spreads, FX, volatility,
//! cross-factor interactions, model parameters, and market scalars.

pub(crate) mod detail;
pub(crate) mod result;

pub(crate) use detail::*;
pub(crate) use result::*;
