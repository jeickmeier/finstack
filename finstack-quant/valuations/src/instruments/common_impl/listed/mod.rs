//! Shared listed-contract terms and generic option-on-future mechanics.

pub(crate) mod future_option;
mod terms;

pub use future_option::{
    FutureOptionExercise, FutureOptionModel, FutureOptionPremiumStyle, FutureOptionSettlement,
    FutureOptionTerms,
};
pub use terms::{ListedDeliveryObligation, ListedFutureSettlement, ListedFutureTerms};

pub(crate) use future_option::impl_future_option_instrument;
