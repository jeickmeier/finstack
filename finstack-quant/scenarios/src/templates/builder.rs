//! Parameterized builder for constructing [`ScenarioSpec`](crate::ScenarioSpec) from templates.

use crate::{HazardBumpMode, OperationSpec, ScenarioEngine, ScenarioSpec};
use finstack_quant_core::market_data::hierarchy::ResolutionMode;

/// A builder for constructing [`ScenarioSpec`] values.
///
/// Template factories return builders pre-populated with conventional curve, surface,
/// equity, and FX identifiers. Consumers can override those identifiers to match their
/// own market data conventions before calling [`build`](Self::build).
#[derive(Debug, Clone)]
pub struct ScenarioSpecBuilder {
    id: String,
    name: Option<String>,
    description: Option<String>,
    operations: Vec<OperationSpec>,
    priority: i32,
    resolution_mode: ResolutionMode,
    hazard_bump_mode: HazardBumpMode,
}

impl ScenarioSpecBuilder {
    /// Create a new builder with the given scenario identifier.
    ///
    /// Prefer calling [`ScenarioSpec::builder`](crate::ScenarioSpec::builder)
    /// from user-facing code so the builder entry point is discoverable from
    /// the built type itself.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable identifier for the scenario that will be built.
    ///
    /// # Returns
    ///
    /// A builder with no operations, default priority `0`, and the default
    /// hierarchy resolution mode.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            operations: Vec::new(),
            priority: 0,
            resolution_mode: ResolutionMode::default(),
            hazard_bump_mode: HazardBumpMode::default(),
        }
    }

    /// Override the scenario identifier.
    ///
    /// # Arguments
    ///
    /// - `id`: Replacement scenario identifier.
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the human-readable scenario name.
    ///
    /// # Arguments
    ///
    /// - `name`: Display name to store in the final [`ScenarioSpec`].
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the optional scenario description.
    ///
    /// # Arguments
    ///
    /// - `description`: Freeform text describing the scenario intent.
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the composition priority. Lower values are applied first.
    ///
    /// # Arguments
    ///
    /// - `priority`: Ordering key used by [`crate::ScenarioEngine::try_compose`].
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set how overlapping hierarchy-targeted operations should resolve.
    ///
    /// # Arguments
    ///
    /// - `resolution_mode`: Hierarchy merge policy to store in the final
    ///   [`ScenarioSpec`].
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn resolution_mode(mut self, resolution_mode: ResolutionMode) -> Self {
        self.resolution_mode = resolution_mode;
        self
    }

    /// Set how ParCDS operations deliver a spread shock onto hazard curves.
    ///
    /// # Arguments
    ///
    /// - `hazard_bump_mode`: Solve-to-par bootstrap (default) or first-order
    ///   hazard-knot shift applied to every ParCDS operation in the spec.
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn hazard_bump_mode(mut self, hazard_bump_mode: HazardBumpMode) -> Self {
        self.hazard_bump_mode = hazard_bump_mode;
        self
    }

    /// Append a single operation to the builder.
    ///
    /// # Arguments
    ///
    /// - `operation`: Scenario operation to append in insertion order.
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn with_operation(mut self, operation: OperationSpec) -> Self {
        self.operations.push(operation);
        self
    }

    /// Append multiple operations to the builder.
    ///
    /// # Arguments
    ///
    /// - `operations`: Operations to append in insertion order.
    ///
    /// # Returns
    ///
    /// The updated builder for fluent chaining.
    pub fn with_operations(mut self, operations: Vec<OperationSpec>) -> Self {
        self.operations.extend(operations);
        self
    }

    /// Compose multiple builders into a single builder.
    ///
    /// The composed builder inherits the engine defaults, including the default `"composed"`
    /// identifier, so callers can override it with [`id`](Self::id) when needed. All
    /// builders must use the same `hazard_bump_mode`.
    ///
    /// # Arguments
    ///
    /// - `builders`: Builders to convert into specs and compose in priority order.
    ///
    /// # Returns
    ///
    /// A new builder containing the composed operations.
    ///
    /// # Errors
    ///
    /// Returns a validation error when `builders` have conflicting
    /// `hazard_bump_mode` values or would produce multiple time-roll operations.
    pub fn compose(builders: Vec<ScenarioSpecBuilder>) -> crate::Result<Self> {
        let specs = builders
            .into_iter()
            .map(ScenarioSpecBuilder::into_spec_without_validation)
            .collect();
        let composed = ScenarioEngine::new().try_compose(specs)?;

        Ok(Self {
            id: composed.id,
            name: composed.name,
            description: composed.description,
            operations: composed.operations,
            priority: composed.priority,
            resolution_mode: composed.resolution_mode,
            hazard_bump_mode: composed.hazard_bump_mode,
        })
    }

    /// Resolve overrides and validate the resulting [`ScenarioSpec`].
    ///
    /// # Returns
    ///
    /// A validated [`ScenarioSpec`] with all configured identifier overrides
    /// applied.
    ///
    /// # Errors
    ///
    /// Returns any validation error raised by [`ScenarioSpec::validate`], such
    /// as empty identifiers, invalid operations, or multiple time-roll
    /// operations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_scenarios::{CurveKind, OperationSpec, ScenarioSpecBuilder};
    ///
    /// let spec = ScenarioSpecBuilder::new("rates")
    ///     .name("Parallel +25bp")
    ///     .priority(10)
    ///     .with_operation(OperationSpec::CurveParallelBp {
    ///         curve_kind: CurveKind::Discount,
    ///         curve_id: "USD_SOFR".into(),
    ///         discount_curve_id: None,
    ///         bp: 25.0,
    ///     })
    ///     .build()?;
    ///
    /// assert_eq!(spec.id, "rates");
    /// assert_eq!(spec.priority, 10);
    /// assert_eq!(spec.operations.len(), 1);
    /// # Ok::<(), finstack_quant_scenarios::Error>(())
    /// ```
    pub fn build(self) -> crate::Result<ScenarioSpec> {
        let spec = ScenarioSpec {
            id: self.id,
            name: self.name,
            description: self.description,
            operations: self.operations,
            priority: self.priority,
            resolution_mode: self.resolution_mode,
            hazard_bump_mode: self.hazard_bump_mode,
        };
        spec.validate()?;
        Ok(spec)
    }

    fn into_spec_without_validation(self) -> ScenarioSpec {
        ScenarioSpec {
            id: self.id,
            name: self.name,
            description: self.description,
            operations: self.operations,
            priority: self.priority,
            resolution_mode: self.resolution_mode,
            hazard_bump_mode: self.hazard_bump_mode,
        }
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::{CurveKind, OperationSpec};
    use finstack_quant_core::market_data::hierarchy::ResolutionMode;

    #[test]
    fn test_builder_basic_construction() {
        let builder = ScenarioSpecBuilder::new("test_scenario")
            .name("Test Scenario")
            .description("A test scenario")
            .priority(5);

        let spec = builder.build().expect("should build");
        assert_eq!(spec.id, "test_scenario");
        assert_eq!(spec.name.as_deref(), Some("Test Scenario"));
        assert_eq!(spec.description.as_deref(), Some("A test scenario"));
        assert_eq!(spec.priority, 5);
        assert!(spec.operations.is_empty());
    }

    #[test]
    fn test_builder_with_operations() {
        let spec = ScenarioSpecBuilder::new("rates")
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                discount_curve_id: None,
                bp: 100.0,
            })
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Forward,
                curve_id: "EUR-ESTR".into(),
                discount_curve_id: None,
                bp: -50.0,
            })
            .build()
            .expect("should build");

        assert_eq!(spec.operations.len(), 2);
    }

    #[test]
    fn test_builder_preserves_explicit_resolution_mode() {
        let spec = ScenarioSpecBuilder::new("hierarchy")
            .resolution_mode(ResolutionMode::Cumulative)
            .build()
            .expect("should build");

        assert_eq!(spec.resolution_mode, ResolutionMode::Cumulative);
    }

    #[test]
    fn test_builder_preserves_explicit_hazard_bump_mode() {
        let spec = ScenarioSpecBuilder::new("credit")
            .hazard_bump_mode(HazardBumpMode::FirstOrderShift)
            .build()
            .expect("should build");

        assert_eq!(spec.hazard_bump_mode, HazardBumpMode::FirstOrderShift);
    }

    #[test]
    fn test_builder_compose_preserves_resolution_mode() {
        let composed = ScenarioSpecBuilder::compose(vec![
            ScenarioSpecBuilder::new("one").resolution_mode(ResolutionMode::Cumulative),
            ScenarioSpecBuilder::new("two").resolution_mode(ResolutionMode::Cumulative),
        ])
        .expect("same-mode compose should succeed")
        .build()
        .expect("should build");

        assert_eq!(composed.resolution_mode, ResolutionMode::Cumulative);
    }

    #[test]
    fn test_builder_compose() {
        let builder1 = ScenarioSpecBuilder::new("rates")
            .priority(0)
            .with_operation(OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD-SOFR".into(),
                discount_curve_id: None,
                bp: 100.0,
            });

        let builder2 = ScenarioSpecBuilder::new("equity")
            .priority(1)
            .with_operation(OperationSpec::EquityPricePct {
                ids: vec!["SPX".into()],
                pct: -20.0,
            });

        let composed = ScenarioSpecBuilder::compose(vec![builder1, builder2])
            .expect("same-mode compose should succeed")
            .id("hybrid");
        let spec = composed.build().expect("should build");

        assert_eq!(spec.id, "hybrid");
        assert_eq!(spec.operations.len(), 2);
    }

    #[test]
    fn test_builder_compose_rejects_mixed_hazard_bump_modes() {
        let error = ScenarioSpecBuilder::compose(vec![
            ScenarioSpecBuilder::new("first-order")
                .hazard_bump_mode(HazardBumpMode::FirstOrderShift),
            ScenarioSpecBuilder::new("solve-to-par").hazard_bump_mode(HazardBumpMode::SolveToPar),
        ])
        .expect_err("mixed hazard bump modes must be rejected");

        let message = error.to_string();
        assert!(
            message.contains("first-order"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("solve-to-par"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("first_order_shift"),
            "unexpected error: {message}"
        );
        assert!(
            message.contains("solve_to_par"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn test_builder_compose_keeps_agreed_first_order_hazard_mode() {
        let composed = ScenarioSpecBuilder::compose(vec![
            ScenarioSpecBuilder::new("one").hazard_bump_mode(HazardBumpMode::FirstOrderShift),
            ScenarioSpecBuilder::new("two").hazard_bump_mode(HazardBumpMode::FirstOrderShift),
        ])
        .expect("same-mode compose should succeed")
        .build()
        .expect("composed spec should build");

        assert_eq!(composed.hazard_bump_mode, HazardBumpMode::FirstOrderShift);
    }

    #[test]
    fn test_builder_validation_empty_id() {
        let result = ScenarioSpecBuilder::new("").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_operations_batch() {
        let ops = vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "A".into(),
                discount_curve_id: None,
                bp: 10.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "B".into(),
                discount_curve_id: None,
                bp: 20.0,
            },
        ];

        let spec = ScenarioSpecBuilder::new("test")
            .with_operations(ops)
            .build()
            .expect("should build");

        assert_eq!(spec.operations.len(), 2);
    }
}
