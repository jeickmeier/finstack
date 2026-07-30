use finstack_quant_core::{
    ContractDescriptor, ContractError, LoadLimits, DEFAULT_MAX_ARTIFACTS, DEFAULT_MAX_BYTES,
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_DIAGNOSTICS, DEFAULT_MAX_POSITIONS,
};

#[test]
fn descriptor_defaults_to_a_strict_current_version() {
    let descriptor = ContractDescriptor::new("finstack_quant.instrument", 1);

    assert_eq!(descriptor.id, "finstack_quant.instrument");
    assert_eq!(descriptor.current, 1);
    assert_eq!(descriptor.supported, 1..=1);
    assert_eq!(descriptor.legacy_missing, None);
    assert_eq!(descriptor.schema_string(), "finstack_quant.instrument/1");
}

#[test]
fn descriptor_parses_only_its_exact_supported_schema() {
    let mut descriptor = ContractDescriptor::new("finstack_quant.instrument", 3);
    descriptor.supported = 1..=3;

    for (schema, expected) in [
        ("finstack_quant.instrument/1", 1),
        ("finstack_quant.instrument/2", 2),
        ("finstack_quant.instrument/3", 3),
    ] {
        assert_eq!(descriptor.parse_schema(schema).unwrap(), expected);
    }
}

#[test]
fn descriptor_rejects_malformed_alias_zero_and_future_schemas() {
    let descriptor = ContractDescriptor::new("finstack_quant.instrument", 1);

    for malformed in [
        "",
        "finstack_quant.instrument",
        "finstack_quant.instrument/",
        "finstack_quant.instrument/not-a-version",
        "finstack_quant.instrument/1/extra",
        "finstack_quant.instruments/1",
    ] {
        assert!(
            matches!(
                descriptor.parse_schema(malformed),
                Err(ContractError::MalformedSchema { .. })
            ),
            "{malformed:?} should be malformed"
        );
    }

    for unsupported in ["finstack_quant.instrument/0", "finstack_quant.instrument/2"] {
        assert!(
            matches!(
                descriptor.parse_schema(unsupported),
                Err(ContractError::UnsupportedVersion {
                    contract,
                    found,
                    min: 1,
                    max: 1,
                }) if contract == "finstack_quant.instrument"
                    && found == unsupported.rsplit_once('/').unwrap().1.parse::<u32>().unwrap()
            ),
            "{unsupported:?} should be unsupported"
        );
    }
}

#[test]
fn descriptor_resolves_missing_versions_by_explicit_policy() {
    let strict = ContractDescriptor::new("finstack_quant.instrument", 2);
    assert!(matches!(
        strict.resolve(None),
        Err(ContractError::MissingVersion { contract })
            if contract == "finstack_quant.instrument"
    ));
    assert_eq!(strict.resolve(Some(2)).unwrap(), 2);

    let mut legacy = strict;
    legacy.supported = 1..=2;
    legacy.legacy_missing = Some(1);
    assert_eq!(legacy.resolve(None).unwrap(), 1);
}

#[test]
fn load_limits_defaults_and_builders_are_explicit() {
    let defaults = LoadLimits::default();
    assert_eq!(defaults.max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(defaults.max_artifacts, DEFAULT_MAX_ARTIFACTS);
    assert_eq!(defaults.max_positions, DEFAULT_MAX_POSITIONS);
    assert_eq!(defaults.max_depth, DEFAULT_MAX_DEPTH);
    assert_eq!(defaults.max_diagnostics, DEFAULT_MAX_DIAGNOSTICS);

    let custom = LoadLimits::default()
        .with_max_bytes(1)
        .with_max_artifacts(2)
        .with_max_positions(3)
        .with_max_depth(4)
        .with_max_diagnostics(5);
    assert_eq!(custom.max_bytes, 1);
    assert_eq!(custom.max_artifacts, 2);
    assert_eq!(custom.max_positions, 3);
    assert_eq!(custom.max_depth, 4);
    assert_eq!(custom.max_diagnostics, 5);
}
