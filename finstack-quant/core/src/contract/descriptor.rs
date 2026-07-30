//! Identity and version policy for persisted contracts.

use std::ops::RangeInclusive;

use super::{ContractError, Diagnostic, LoadLimits, LoadPhase, Severity, ValidationReport};

/// Identity and version policy for one persisted contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractDescriptor {
    /// Stable contract identifier, such as `"finstack_quant.instrument"`.
    pub id: &'static str,
    /// Version written by current producers.
    pub current: u32,
    /// Inclusive range of versions accepted by the loader.
    pub supported: RangeInclusive<u32>,
    /// Version assigned to payloads that omit the version marker.
    ///
    /// `None` means a missing version is an error, which is the strict default
    /// for new contracts.
    pub legacy_missing: Option<u32>,
}

impl ContractDescriptor {
    /// Create a strict descriptor that supports only its current version.
    ///
    /// # Arguments
    ///
    /// * `id` - Stable, exact contract identifier used before the slash in
    ///   schema strings.
    /// * `current` - Positive version emitted by current producers and accepted
    ///   as the descriptor's sole supported version.
    #[must_use]
    pub const fn new(id: &'static str, current: u32) -> Self {
        Self {
            id,
            current,
            supported: current..=current,
            legacy_missing: None,
        }
    }

    /// Return the current schema marker as `"<contract>/<version>"`.
    #[must_use]
    pub fn schema_string(&self) -> String {
        format!("{}/{}", self.id, self.current)
    }

    /// Parse and validate an exact schema marker for this contract.
    ///
    /// The contract identifier must match exactly. Version zero, versions
    /// outside [`Self::supported`], missing components, and non-decimal version
    /// text are rejected.
    ///
    /// # Arguments
    ///
    /// * `s` - Schema marker in the exact `"<contract>/<version>"` format.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::MalformedSchema`] when the marker is malformed
    /// or names another contract, and [`ContractError::UnsupportedVersion`]
    /// when its parsed version is zero or outside the supported range.
    pub fn parse_schema(&self, s: &str) -> Result<u32, ContractError> {
        let expected = self.id.to_string();
        let Some(version_text) = s
            .strip_prefix(self.id)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            return Err(ContractError::MalformedSchema {
                value: s.to_string(),
                expected,
            });
        };

        if version_text.is_empty() || version_text.contains('/') {
            return Err(ContractError::MalformedSchema {
                value: s.to_string(),
                expected,
            });
        }

        let version = version_text
            .parse::<u32>()
            .map_err(|_| ContractError::MalformedSchema {
                value: s.to_string(),
                expected,
            })?;
        self.validate_version(version)
    }

    /// Resolve an optional payload version under this descriptor's policy.
    ///
    /// # Arguments
    ///
    /// * `found` - Explicit payload version, or `None` when the payload omitted
    ///   its version marker. Missing versions use [`Self::legacy_missing`] when
    ///   configured.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::MissingVersion`] under strict missing-version
    /// policy, or [`ContractError::UnsupportedVersion`] when the resolved
    /// version is zero or outside [`Self::supported`].
    pub fn resolve(&self, found: Option<u32>) -> Result<u32, ContractError> {
        match found.or(self.legacy_missing) {
            Some(version) => self.validate_version(version),
            None => Err(ContractError::MissingVersion {
                contract: self.id.to_string(),
            }),
        }
    }

    /// Resolve a required explicit numeric version and return structured errors.
    ///
    /// Unlike [`Self::resolve`], this method deliberately ignores
    /// [`Self::legacy_missing`]. It is intended for strict persistence paths
    /// where accepting an omitted version would make the stored contract
    /// ambiguous.
    ///
    /// # Arguments
    ///
    /// * `found` - Explicit numeric version read from the payload, or `None`
    ///   when its version key was absent.
    /// * `pointer` - JSON Pointer locating the numeric version key.
    /// * `limits` - Resource policy bounding retained diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::Report`] with stable code
    /// `"contract/version-missing"` or `"contract/version-unsupported"`.
    pub fn resolve_strict(
        &self,
        found: Option<u32>,
        pointer: &str,
        limits: &LoadLimits,
    ) -> Result<u32, ContractError> {
        let result = match found {
            Some(version) => self.validate_version(version),
            None => Err(ContractError::MissingVersion {
                contract: self.id.to_string(),
            }),
        };
        result.map_err(|error| self.diagnostic_error(error, pointer, limits))
    }

    /// Parse a required schema marker and return structured contract errors.
    ///
    /// # Arguments
    ///
    /// * `found` - Exact schema marker from the payload, or `None` when the
    ///   schema key was absent.
    /// * `pointer` - JSON Pointer locating the schema marker.
    /// * `limits` - Resource policy bounding retained diagnostics.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::Report`] with stable code
    /// `"contract/version-missing"`, `"contract/version-unsupported"`, or
    /// `"contract/schema-malformed"`.
    pub fn parse_schema_strict(
        &self,
        found: Option<&str>,
        pointer: &str,
        limits: &LoadLimits,
    ) -> Result<u32, ContractError> {
        let result = found.map_or_else(
            || {
                Err(ContractError::MissingVersion {
                    contract: self.id.to_string(),
                })
            },
            |schema| self.parse_schema(schema),
        );
        result.map_err(|error| self.diagnostic_error(error, pointer, limits))
    }

    fn validate_version(&self, version: u32) -> Result<u32, ContractError> {
        if version != 0 && self.supported.contains(&version) {
            return Ok(version);
        }

        Err(ContractError::UnsupportedVersion {
            contract: self.id.to_string(),
            found: version,
            min: *self.supported.start(),
            max: *self.supported.end(),
        })
    }

    fn diagnostic_error(
        &self,
        error: ContractError,
        pointer: &str,
        limits: &LoadLimits,
    ) -> ContractError {
        let diagnostic = match &error {
            ContractError::MissingVersion { .. } => Diagnostic::new(
                "contract/version-missing",
                LoadPhase::Version,
                Severity::Error,
                error.to_string(),
            )
            .with_pointer(pointer)
            .with_contract(self.id)
            .with_expected_version(self.current),
            ContractError::UnsupportedVersion { found, .. } => Diagnostic::new(
                "contract/version-unsupported",
                LoadPhase::Version,
                Severity::Error,
                error.to_string(),
            )
            .with_pointer(pointer)
            .with_contract(self.id)
            .with_expected_version(self.current)
            .with_actual_version(*found),
            ContractError::MalformedSchema { .. } => Diagnostic::new(
                "contract/schema-malformed",
                LoadPhase::Version,
                Severity::Error,
                error.to_string(),
            )
            .with_pointer(pointer)
            .with_contract(self.id)
            .with_expected_version(self.current),
            _ => return error,
        };
        let mut report = ValidationReport::default();
        report.push_bounded(limits, diagnostic);
        ContractError::Report(Box::new(report))
    }
}
