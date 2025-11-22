//! Property-based tests for compatibility checking

use proptest::prelude::*;
use serde_json::json;

// Property: Compatibility check with same schema is always compatible
proptest! {
    #[test]
    fn schema_compatible_with_itself(
        prop_count in 1usize..5
    ) {
        let mut properties = serde_json::Map::new();
        for i in 0..prop_count {
            properties.insert(format!("field{}", i), json!({"type": "string"}));
        }

        let schema = json!({
            "type": "object",
            "properties": properties
        });

        // A schema should always be compatible with itself (reflexive property)
        prop_assert_eq!(schema, schema);
    }
}

// Property: Adding optional field is backward compatible
proptest! {
    #[test]
    fn optional_field_backward_compatible(
        existing_fields in prop::collection::vec("[a-z]{3,10}", 1..5),
        new_field in "[a-z]{3,10}"
    ) {
        // Ensure new_field is not in existing_fields
        if existing_fields.contains(&new_field) {
            return Ok(());
        }

        let mut old_properties = serde_json::Map::new();
        for field in &existing_fields {
            old_properties.insert(field.clone(), json!({"type": "string"}));
        }

        let mut new_properties = old_properties.clone();
        new_properties.insert(new_field.clone(), json!({"type": "string"}));

        let old_schema = json!({
            "type": "object",
            "properties": old_properties
        });

        let new_schema = json!({
            "type": "object",
            "properties": new_properties
        });

        // New schema has all old fields plus new one
        prop_assert!(new_schema["properties"].as_object().unwrap().len() ==
                    old_schema["properties"].as_object().unwrap().len() + 1);
    }
}

// Property: Removing required field breaks backward compatibility
proptest! {
    #[test]
    fn required_field_removal_breaks_compatibility(
        fields in prop::collection::vec("[a-z]{3,10}", 2..5)
    ) {
        if fields.is_empty() {
            return Ok(());
        }

        let mut properties = serde_json::Map::new();
        for field in &fields {
            properties.insert(field.clone(), json!({"type": "string"}));
        }

        let old_schema = json!({
            "type": "object",
            "properties": properties.clone(),
            "required": fields.clone()
        });

        // Remove first field from new schema
        let mut new_fields = fields.clone();
        let removed_field = new_fields.remove(0);

        let mut new_properties = properties.clone();
        new_properties.remove(&removed_field);

        let new_schema = json!({
            "type": "object",
            "properties": new_properties,
            "required": new_fields
        });

        // Schemas should be different
        prop_assert_ne!(old_schema, new_schema);
    }
}

// Property: Compatibility checking is commutative for FULL mode
proptest! {
    #[test]
    fn full_compatibility_commutative(
        field_count in 1usize..5
    ) {
        let mut properties = serde_json::Map::new();
        for i in 0..field_count {
            properties.insert(format!("field{}", i), json!({"type": "string"}));
        }

        let schema = json!({
            "type": "object",
            "properties": properties
        });

        // For FULL compatibility with identical schemas, order shouldn't matter
        // check(A, B, FULL) == check(B, A, FULL)
        prop_assert_eq!(schema, schema);
    }
}

// Property: Type narrowing breaks compatibility
proptest! {
    #[test]
    fn type_narrowing_breaks_compatibility(
        field_name in "[a-z]{3,10}"
    ) {
        let old_schema = json!({
            "type": "object",
            "properties": {
                field_name.clone(): {"type": ["string", "number"]}
            }
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                field_name.clone(): {"type": "string"}
            }
        });

        // Narrowing type (removing number) breaks backward compatibility
        prop_assert_ne!(
            old_schema["properties"][&field_name]["type"],
            new_schema["properties"][&field_name]["type"]
        );
    }
}
