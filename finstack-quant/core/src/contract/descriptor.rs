//! Identity and version policy for persisted contracts.

use super::{ContractError, Diagnostic, LoadLimits, LoadPhase, Severity, ValidationReport};

/// Identity and version policy for one persisted contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractDescriptor {
    /// Stable contract identifier, such as `"finstack_quant.instrument"`.
    pub id: &'static str,
}

impl ContractDescriptor {
    /// The sole version emitted and accepted by all persisted contracts.
    pub const VERSION: u32 = 1;

    /// Create a strict v1 descriptor.
    ///
    /// # Arguments
    ///
    /// * `id` - Stable, exact contract identifier used before the slash in
    ///   schema strings.
    #[must_use]
    pub const fn new(id: &'static str) -> Self {
        Self { id }
    }

    /// Return the current schema marker as `"<contract>/<version>"`.
    #[must_use]
    pub fn schema_string(&self) -> String {
        format!("{}/{}", self.id, Self::VERSION)
    }

    /// Parse and validate an exact schema marker for this contract.
    ///
    /// The contract identifier must match exactly. Any version other than v1,
    /// missing components, and non-decimal version text are rejected.
    ///
    /// # Arguments
    ///
    /// * `s` - Schema marker in the exact `"<contract>/<version>"` format.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::MalformedSchema`] when the marker is malformed
    /// or names another contract, and [`ContractError::UnsupportedVersion`]
    /// when its parsed version is not v1.
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

    /// Resolve a required payload version.
    ///
    /// # Arguments
    ///
    /// * `found` - Explicit payload version, or `None` when the payload omitted
    ///   its version marker.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::MissingVersion`] under strict missing-version
    /// policy, or [`ContractError::UnsupportedVersion`] when the resolved
    /// version is not v1.
    pub fn resolve(&self, found: Option<u32>) -> Result<u32, ContractError> {
        match found {
            Some(version) => self.validate_version(version),
            None => Err(ContractError::MissingVersion {
                contract: self.id.to_string(),
            }),
        }
    }

    /// Resolve a required explicit numeric version and return structured errors.
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
        if version == Self::VERSION {
            return Ok(version);
        }

        Err(ContractError::UnsupportedVersion {
            contract: self.id.to_string(),
            found: version,
            min: Self::VERSION,
            max: Self::VERSION,
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
            .with_expected_version(Self::VERSION),
            ContractError::UnsupportedVersion { found, .. } => Diagnostic::new(
                "contract/version-unsupported",
                LoadPhase::Version,
                Severity::Error,
                error.to_string(),
            )
            .with_pointer(pointer)
            .with_contract(self.id)
            .with_expected_version(Self::VERSION)
            .with_actual_version(*found),
            ContractError::MalformedSchema { .. } => Diagnostic::new(
                "contract/schema-malformed",
                LoadPhase::Version,
                Severity::Error,
                error.to_string(),
            )
            .with_pointer(pointer)
            .with_contract(self.id)
            .with_expected_version(Self::VERSION),
            _ => return error,
        };
        let mut report = ValidationReport::default();
        report.push_bounded(limits, diagnostic);
        ContractError::Report(Box::new(report))
    }
}
