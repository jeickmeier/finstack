//! Cashflow module integration tests.
//!
//! This test suite verifies market-standard correctness for:
//! - CashFlow struct construction and validation
//! - NPV/discounting calculations
//! - XIRR/IRR calculations with reference golden values
//!
//! # Test Organization
//!
//! - `primitives`: CashFlow struct construction and validation
//! - `discounting`: NPV calculations and discount factor properties
//! - `irr`: IRR/XIRR golden values, edge cases, and input validation

#[path = "cashflow/discounting.rs"]
mod discounting;

#[path = "cashflow/irr.rs"]
mod irr;

#[path = "cashflow/primitives.rs"]
mod primitives;
