//! Cashflows test suite modules.
//!
//! This module consolidates:
//! - `provider_contract`: `CashflowProvider` trait contract compliance
//! - `bridge_smoke`: compatibility smoke tests for `finstack_quant_cashflows::*`

pub(crate) mod helpers;

#[path = "../support/rates.rs"]
#[allow(dead_code, unused_imports)]
pub(crate) mod rates_support;

mod bridge_smoke;
mod instrument_bridge;
mod provider_contract;
