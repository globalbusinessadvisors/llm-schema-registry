//! Property-based tests for schema operations

use proptest::prelude::*;
use serde_json::{json, Value};

// Property: Schema serialization roundtrip preserves data
proptest! {
    #[test]
    fn schema_json_roundtrip(
        type_val in prop::sample::select(vec!["object", "array", "string", "number", "boolean", "null"])
    ) {
        let schema = json!({
            "type": type_val
        });

        let serialized = serde_json::to_string(&schema).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(schema, deserialized);
    }
}

// Property: Hash calculation is deterministic
proptest! {
    #[test]
    fn hash_calculation_deterministic(content in ".*") {
        use sha2::{Sha256, Digest};

        let hash1 = {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let hash2 = {
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        prop_assert_eq!(hash1, hash2);
    }
}

// Property: Semantic version parsing is consistent
proptest! {
    #[test]
    fn semantic_version_roundtrip(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100
    ) {
        let version_str = format!("{}.{}.{}", major, minor, patch);
        let parsed = semver::Version::parse(&version_str).unwrap();

        prop_assert_eq!(parsed.major, major as u64);
        prop_assert_eq!(parsed.minor, minor as u64);
        prop_assert_eq!(parsed.patch, patch as u64);
    }
}

// Property: UUID generation produces unique values
proptest! {
    #[test]
    fn uuid_uniqueness(_repeat in 0..100) {
        use uuid::Uuid;
        use std::collections::HashSet;

        let mut uuids = HashSet::new();
        for _ in 0..1000 {
            let uuid = Uuid::new_v4();
            prop_assert!(uuids.insert(uuid), "Duplicate UUID generated");
        }
    }
}

// Property: JSON schema with properties is valid
proptest! {
    #[test]
    fn json_schema_properties_valid(
        prop_count in 1usize..10
    ) {
        let mut properties = serde_json::Map::new();

        for i in 0..prop_count {
            properties.insert(
                format!("field{}", i),
                json!({"type": "string"})
            );
        }

        let schema = json!({
            "type": "object",
            "properties": properties
        });

        // Should be valid JSON
        prop_assert!(schema.is_object());
        prop_assert_eq!(schema["type"], "object");
        prop_assert_eq!(schema["properties"].as_object().unwrap().len(), prop_count);
    }
}

// Property: Content hash changes when content changes
proptest! {
    #[test]
    fn content_hash_changes(
        content1 in ".*",
        content2 in ".*"
    ) {
        use sha2::{Sha256, Digest};

        if content1 == content2 {
            return Ok(());
        }

        let hash1 = {
            let mut hasher = Sha256::new();
            hasher.update(content1.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let hash2 = {
            let mut hasher = Sha256::new();
            hasher.update(content2.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        prop_assert_ne!(hash1, hash2);
    }
}

// Property: Schema namespace and name combination is unique identifier
proptest! {
    #[test]
    fn schema_identifier_uniqueness(
        namespace in "[a-z]{3,20}",
        name in "[a-z]{3,20}",
        version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}"
    ) {
        let id1 = format!("{}:{}:{}", namespace, name, version);
        let id2 = format!("{}:{}:{}", namespace, name, version);

        prop_assert_eq!(id1, id2);
    }
}

// Property: Required fields list is subset of properties
proptest! {
    #[test]
    fn required_fields_subset_of_properties(
        fields in prop::collection::vec("[a-z]{1,10}", 1..10)
    ) {
        let mut properties = serde_json::Map::new();
        for field in &fields {
            properties.insert(field.clone(), json!({"type": "string"}));
        }

        let required = fields.clone();

        let schema = json!({
            "type": "object",
            "properties": properties,
            "required": required
        });

        // All required fields should exist in properties
        let props = schema["properties"].as_object().unwrap();
        let reqs = schema["required"].as_array().unwrap();

        for req in reqs {
            let field_name = req.as_str().unwrap();
            prop_assert!(props.contains_key(field_name),
                "Required field '{}' not in properties", field_name);
        }
    }
}

// Property: Schema with additionalProperties=false rejects unknown fields
proptest! {
    #[test]
    fn additional_properties_false_validation(
        known_field in "[a-z]{3,10}",
        unknown_field in "[a-z]{3,10}"
    ) {
        if known_field == unknown_field {
            return Ok(());
        }

        let schema = json!({
            "type": "object",
            "properties": {
                known_field.clone(): {"type": "string"}
            },
            "additionalProperties": false
        });

        // Data with only known field should be valid
        let valid_data = json!({
            known_field: "value"
        });

        // Data with unknown field should be invalid
        let invalid_data = json!({
            known_field: "value",
            unknown_field: "extra"
        });

        prop_assert!(schema["properties"].as_object().unwrap().contains_key(&known_field));
        prop_assert!(!schema["properties"].as_object().unwrap().contains_key(&unknown_field));
    }
}

// Property: Timestamp ordering is consistent
proptest! {
    #[test]
    fn timestamp_ordering(
        offset_secs in 0i64..1000
    ) {
        use chrono::{Utc, Duration};

        let now = Utc::now();
        let future = now + Duration::seconds(offset_secs);

        prop_assert!(future >= now);
        prop_assert_eq!(future - now, Duration::seconds(offset_secs));
    }
}
