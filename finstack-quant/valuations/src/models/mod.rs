//! Option pricing models and numerical methods with academic foundations.
//!
//! Provides reusable pricing models for options and derivatives, including
//! closed-form formulas, tree-based methods, volatility models, and Black-Scholes variants.
//! All implementations cite their academic sources for correctness verification.
//!
//! # Module Organization
//!
//! - `closed_form`: Closed-form and semi-analytical pricing formulas (Black-Scholes Greeks,
//!   Asian, Barrier, Lookback, Quanto, Heston)
//! - `volatility`: Volatility models (SABR) and Black-Scholes helper functions
//! - `trees`: Tree-based methods (Binomial, Trinomial, Multi-factor, Short-rate)
//! - `pde`: Finite difference PDE methods (1D Crank-Nicolson, 2D Craig-Sneyd ADI, Heston, American penalty)
//!
//! Credit copulas, recovery models, and factor models live in [`crate::correlation`].

pub mod closed_form;
pub mod credit;
pub mod pde;
pub mod trees;
pub mod volatility;

pub(crate) use closed_form::{black76_call, black76_put};
pub use closed_form::{
    black76_implied_vol, bs_greeks, bs_greeks_checked, bs_implied_vol, bs_price, bs_price_checked,
    heston_call_price_fourier, heston_put_price_fourier, vanilla_expiry_payoff, BsGreeks,
    HestonParams, ONE_PERCENT,
};
pub use pde::{
    BlackScholesPde, BoundaryCondition, CraigSneydStepper, Grid1D, Grid2D, HestonPde, PdeProblem1D,
    PdeProblem2D, PdeSolution, PdeSolution2D, Solver1D, Solver2D,
};
pub use trees::{
    short_rate_keys, single_factor_equity_state, state_keys, two_factor_equity_rates_state,
    BarrierSpec, BarrierStyle, BinomialTree, EvolutionParams, HullWhiteTree, HullWhiteTreeConfig,
    NodeState, ShortRateModel, ShortRateTree, ShortRateTreeConfig, TreeBranching, TreeCompounding,
    TreeGreeks, TreeModel, TreeParameters, TreeType, TreeValuator,
};
pub(crate) use volatility::{d1, d1_black76, d1_d2_black76, d2_black76, vega_weight};
pub use volatility::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};
