//! Strict serialization tests for dynamic term-structure models.

use finstack_quant_models::rates::dtsm::DieboldLi;

fn assert_strict_inbound<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_value(value).unwrap();
    let _: T = serde_json::from_value(json.clone()).unwrap();

    let mut tampered = json;
    tampered
        .as_object_mut()
        .expect("serialized state should be a JSON object")
        .insert("typo_field".to_string(), serde_json::json!(1));
    assert!(serde_json::from_value::<T>(tampered).is_err());
}

#[test]
fn diebold_li_rejects_unknown_fields() {
    let model = DieboldLi::with_default_lambda();
    assert_strict_inbound(&model);
}

#[test]
fn diebold_li_rejects_invalid_lambda_on_deserialize() {
    let model = DieboldLi::with_default_lambda();
    let mut json = serde_json::to_value(&model).unwrap();
    json.as_object_mut()
        .unwrap()
        .insert("lambda".to_string(), serde_json::json!(-1.0));
    assert!(serde_json::from_value::<DieboldLi>(json).is_err());
}
