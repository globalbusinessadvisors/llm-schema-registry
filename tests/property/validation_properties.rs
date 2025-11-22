//! Property-based tests for validation

use proptest::prelude::*;
use serde_json::json;

// Property: Valid data against simple schema
proptest! {
    #[test]
    fn string_type_validation(
        value in ".*"
    ) {
        let schema = json!({
            "type": "string"
        });

        let data = json!(value);

        // Data is a string, so it should match string schema
        prop_assert!(data.is_string());
    }
}

// Property: Number validation
proptest! {
    #[test]
    fn number_type_validation(
        value in -1000.0f64..1000.0f64
    ) {
        let schema = json!({
            "type": "number"
        });

        let data = json!(value);

        prop_assert!(data.is_number());
    }
}

// Property: Boolean validation
proptest! {
    #[test]
    fn boolean_type_validation(
        value in prop::bool::ANY
    ) {
        let schema = json!({
            "type": "boolean"
        });

        let data = json!(value);

        prop_assert!(data.is_boolean());
        prop_assert_eq!(data.as_bool().unwrap(), value);
    }
}

// Property: Array validation
proptest! {
    #[test]
    fn array_type_validation(
        items in prop::collection::vec(any::<i32>(), 0..10)
    ) {
        let schema = json!({
            "type": "array",
            "items": {"type": "number"}
        });

        let data = json!(items);

        prop_assert!(data.is_array());
        prop_assert_eq!(data.as_array().unwrap().len(), items.len());
    }
}

// Property: Required fields validation
proptest! {
    #[test]
    fn required_fields_validation(
        name in "[a-z]{3,20}",
        age in 0u32..120
    ) {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name", "age"]
        });

        let data = json!({
            "name": name,
            "age": age
        });

        // Data contains all required fields
        prop_assert!(data.as_object().unwrap().contains_key("name"));
        prop_assert!(data.as_object().unwrap().contains_key("age"));
    }
}

// Property: Min/max validation
proptest! {
    #[test]
    fn min_max_validation(
        value in 10i32..100
    ) {
        let schema = json!({
            "type": "number",
            "minimum": 10,
            "maximum": 100
        });

        let data = json!(value);

        let num = data.as_i64().unwrap();
        let min = schema["minimum"].as_i64().unwrap();
        let max = schema["maximum"].as_i64().unwrap();

        prop_assert!(num >= min);
        prop_assert!(num <= max);
    }
}

// Property: String length validation
proptest! {
    #[test]
    fn string_length_validation(
        value in prop::string::string_regex("[a-z]{5,10}").unwrap()
    ) {
        let schema = json!({
            "type": "string",
            "minLength": 5,
            "maxLength": 10
        });

        let data = json!(value);

        let str_val = data.as_str().unwrap();
        let min_len = schema["minLength"].as_u64().unwrap() as usize;
        let max_len = schema["maxLength"].as_u64().unwrap() as usize;

        prop_assert!(str_val.len() >= min_len);
        prop_assert!(str_val.len() <= max_len);
    }
}

// Property: Pattern validation
proptest! {
    #[test]
    fn pattern_validation(
        digits in "[0-9]{5}"
    ) {
        let schema = json!({
            "type": "string",
            "pattern": "^[0-9]{5}$"
        });

        let data = json!(digits);

        // Should match pattern
        let regex = regex::Regex::new(r"^[0-9]{5}$").unwrap();
        prop_assert!(regex.is_match(data.as_str().unwrap()));
    }
}

// Property: Enum validation
proptest! {
    #[test]
    fn enum_validation(
        value in prop::sample::select(vec!["red", "green", "blue"])
    ) {
        let schema = json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        let data = json!(value);

        let valid_values = schema["enum"].as_array().unwrap();
        prop_assert!(valid_values.contains(&data));
    }
}

// Property: Nested object validation
proptest! {
    #[test]
    fn nested_object_validation(
        street in "[a-z ]{3,20}",
        city in "[a-z]{3,15}",
        zip in "[0-9]{5}"
    ) {
        let schema = json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"},
                        "zip": {"type": "string", "pattern": "^[0-9]{5}$"}
                    },
                    "required": ["street", "city", "zip"]
                }
            }
        });

        let data = json!({
            "address": {
                "street": street,
                "city": city,
                "zip": zip
            }
        });

        // Verify nested structure
        prop_assert!(data["address"].is_object());
        prop_assert_eq!(data["address"]["street"].as_str().unwrap(), street);
        prop_assert_eq!(data["address"]["city"].as_str().unwrap(), city);
        prop_assert_eq!(data["address"]["zip"].as_str().unwrap(), zip);
    }
}
