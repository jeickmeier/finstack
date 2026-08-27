//! Generic validation helpers for checking invariants.
//!
//! These helpers are convention-agnostic: they enforce structural invariants
//! (conditions, ordering, finiteness) without encoding market-specific defaults.

/// Require a condition to be true, otherwise return a validation error.
///
/// This is a concise guard for structural or domain invariants. `message` is
/// converted only when `condition` is false; use [`require_with`] when forming
/// that message itself is expensive.
///
/// # Arguments
///
/// * `condition` - Invariant or validation predicate that must evaluate to
///   `true` for success.
/// * `message` - Error text converted to an owned string only when `condition`
///   is false.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] containing `message` when `condition`
/// is false.
#[inline]
pub fn require(condition: bool, message: impl Into<String>) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(crate::Error::Validation(message.into()))
    }
}

/// Require a condition to be true, otherwise return the provided error.
///
/// Use this when a caller needs to preserve a domain-specific error variant
/// rather than collapse a failed predicate into `Error::Validation`.
///
/// # Arguments
///
/// * `condition` - Invariant or validation predicate that must evaluate to
///   `true` for success.
/// * `err` - Domain-specific error converted and returned when `condition` is
///   false.
///
/// # Errors
///
/// Returns `err.into()` when `condition` is false, preserving the supplied
/// error's category and diagnostic information.
#[inline]
pub fn require_or(condition: bool, err: impl Into<crate::Error>) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(err.into())
    }
}

/// Require a condition to be true, lazily constructing the error message.
///
/// The closure is never invoked when the condition holds, making this helper
/// appropriate when an error message needs formatting or data collection.
///
/// # Arguments
///
/// * `condition` - Invariant or validation predicate that must evaluate to
///   `true` for success.
/// * `message` - Closure that lazily constructs validation text only when
///   `condition` is false.
///
/// # Errors
///
/// Invokes `message` and returns [`crate::Error::Validation`] with its result
/// when `condition` is false. If `message` panics, that panic propagates.
#[inline]
pub fn require_with(condition: bool, message: impl FnOnce() -> String) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(crate::Error::Validation(message()))
    }
}

/// Validate that a floating-point value is finite.
///
/// # Arguments
///
/// * `value` - Floating-point input that must not be NaN or infinite.
/// * `context` - Field or calculation label included in validation diagnostics.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] when `value` is not finite.
#[inline]
pub fn validate_f64_finite(value: f64, context: &str) -> crate::Result<()> {
    require_with(value.is_finite(), || {
        format!("Invalid {context}: must be finite.")
    })
}

/// Validate that a floating-point value is finite and strictly positive.
///
/// # Arguments
///
/// * `value` - Floating-point input that must be greater than zero.
/// * `context` - Field or calculation label included in validation diagnostics.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] when `value` is non-finite or not
/// strictly positive.
#[inline]
pub fn validate_f64_positive(value: f64, context: &str) -> crate::Result<()> {
    require_with(value.is_finite() && value > 0.0, || {
        format!("Invalid {context}: must be positive, got {value}")
    })
}

/// Validate that a floating-point value is finite and non-negative.
///
/// # Arguments
///
/// * `value` - Floating-point input that must be greater than or equal to zero.
/// * `context` - Field or calculation label included in validation diagnostics.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] when `value` is non-finite or negative.
#[inline]
pub fn validate_f64_non_negative(value: f64, context: &str) -> crate::Result<()> {
    require_with(value.is_finite() && value >= 0.0, || {
        format!("Invalid {context}: must be non-negative, got {value}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, InputError};

    #[test]
    fn require_returns_validation_error_when_condition_is_false() {
        assert!(require(true, "ok").is_ok());

        let err = require(false, "missing invariant").expect_err("false condition should fail");
        assert!(matches!(err, Error::Validation(message) if message == "missing invariant"));
    }

    #[test]
    fn require_or_preserves_caller_supplied_error() {
        assert!(require_or(true, InputError::Invalid).is_ok());

        let err = require_or(false, InputError::Invalid).expect_err("false condition should fail");
        assert!(matches!(err, Error::Input(InputError::Invalid)));
    }

    #[test]
    fn require_with_is_lazy_on_success_and_builds_message_on_failure() {
        assert!(require_with(true, || panic!("message closure must not run")).is_ok());

        let err =
            require_with(false, || "lazy failure".to_string()).expect_err("false should fail");
        assert!(matches!(err, Error::Validation(message) if message == "lazy failure"));
    }
}
