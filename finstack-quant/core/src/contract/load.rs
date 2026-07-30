//! Shared JSON loading helpers for strict persisted contracts.

use serde::de::DeserializeOwned;

use super::{ContractError, Diagnostic, LoadLimits, LoadPhase, Severity, ValidationReport};

/// Check encoded-size and JSON nesting-depth limits without building a tree.
///
/// The depth pass is lexical and allocation-free. It counts object and array
/// delimiters outside strings, honors JSON string escapes, and deliberately
/// does not validate JSON syntax. Callers must still parse the document once
/// into their target type after this check succeeds.
///
/// # Arguments
///
/// * `bytes` - Complete JSON document whose encoded size and lexical nesting
///   depth are checked.
/// * `limits` - Resource policy providing the maximum byte length and nesting
///   depth.
///
/// # Errors
///
/// Returns [`ContractError::LimitExceeded`] when the byte or JSON-depth bound
/// is exceeded. Malformed JSON within those bounds is accepted here and must
/// be rejected by the caller's parser.
pub fn check_json_limits(bytes: &[u8], limits: &LoadLimits) -> Result<(), ContractError> {
    if bytes.len() > limits.max_bytes {
        return Err(ContractError::LimitExceeded {
            what: "bytes",
            found: bytes.len(),
            limit: limits.max_bytes,
        });
    }

    let mut depth = 0usize;
    let mut maximum = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                maximum = maximum.max(depth);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    if maximum > limits.max_depth {
        return Err(ContractError::LimitExceeded {
            what: "JSON depth",
            found: maximum,
            limit: limits.max_depth,
        });
    }
    Ok(())
}

/// Parse bounded UTF-8 JSON bytes into a generic value.
///
/// The encoded byte length and resulting container depth are checked against
/// `limits`. JSON syntax failures are returned as structured parse diagnostics
/// with their source line and column.
///
/// # Arguments
///
/// * `bytes` - Complete UTF-8 JSON document to parse.
/// * `limits` - Resource policy bounding encoded bytes, nesting depth, and
///   retained diagnostics.
///
/// # Errors
///
/// Returns [`ContractError::LimitExceeded`] when the byte or depth bound is
/// exceeded, or [`ContractError::Report`] when JSON parsing fails.
pub fn parse_json_value(
    bytes: &[u8],
    limits: &LoadLimits,
) -> Result<serde_json::Value, ContractError> {
    check_json_limits(bytes, limits)?;
    let value = serde_json::from_slice(bytes).map_err(|error| {
        let mut report = ValidationReport::default();
        report.push_bounded(limits, Diagnostic::from_parse_error(&error, None));
        ContractError::Report(Box::new(report))
    })?;
    Ok(value)
}

/// Deserialize a parsed JSON value with a structured shape diagnostic.
///
/// # Arguments
///
/// * `value` - Parsed JSON document to deserialize into `T`.
/// * `limits` - Resource policy bounding retained diagnostics.
///
/// # Errors
///
/// Returns [`ContractError::Report`] with code
/// `"contract/structure-invalid"` when the document does not match `T`.
pub fn deserialize_json_value<T>(
    value: serde_json::Value,
    limits: &LoadLimits,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| {
        let mut report = ValidationReport::default();
        report.push_bounded(
            limits,
            Diagnostic::new(
                "contract/structure-invalid",
                LoadPhase::Structure,
                Severity::Error,
                error.to_string(),
            ),
        );
        ContractError::Report(Box::new(report))
    })
}
