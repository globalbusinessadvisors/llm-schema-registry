# Schema Registry Validation Engine

A high-performance, production-ready validation engine for JSON Schema, Apache Avro, and Protocol Buffers. Designed specifically for the LLM Schema Registry with enterprise-grade validation capabilities.

## Features

### Multi-Format Support
- **JSON Schema**: Draft 7, Draft 2019-09, Draft 2020-12
- **Apache Avro**: Full Avro schema specification
- **Protocol Buffers**: proto2 and proto3

### 7-Step Validation Pipeline

1. **Structural Validation**: Ensures the schema has valid syntax
2. **Type Validation**: Verifies all types are correct and supported
3. **Semantic Validation**: Checks logical consistency (e.g., required fields exist in properties)
4. **Compatibility Validation**: Validates against existing schema versions (separate API)
5. **Security Validation**: Detects potentially malicious patterns and complexity attacks
6. **Performance Validation**: Ensures schemas won't cause performance issues
7. **Custom Rule Validation**: Extensible validation rules for domain-specific requirements

### LLM-Specific Features

- **Description Validation**: Ensures schemas have adequate documentation for LLM understanding
- **Example Validation**: Checks for example values that help LLMs generate correct data
- **Semantic Tag Checking**: Validates semantic metadata for better LLM categorization

### Performance

- **Target**: <50ms p95 validation latency
- **Thread-Safe**: All validators implement `Send + Sync`
- **Zero Unsafe**: No unsafe code blocks
- **Optimized**: Fail-fast modes and configurable validation depth

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
schema-registry-validation = "0.1.0"
```

## Quick Start

### Basic Validation

```rust
use schema_registry_validation::{ValidationEngine, SchemaFormat};

#[tokio::main]
async fn main() {
    let engine = ValidationEngine::new();

    let schema = r#"{
        "type": "object",
        "description": "A user object",
        "properties": {
            "name": {
                "type": "string",
                "description": "User's name"
            },
            "age": {
                "type": "integer",
                "description": "User's age",
                "minimum": 0
            }
        },
        "required": ["name"]
    }"#;

    let result = engine.validate(schema, SchemaFormat::JsonSchema).await?;

    if result.is_valid {
        println!("✓ Schema is valid!");
    } else {
        for error in result.errors {
            eprintln!("✗ Error: {}", error.message);
            if let Some(location) = error.location {
                eprintln!("  Location: {}", location);
            }
            if let Some(suggestion) = error.suggestion {
                eprintln!("  Suggestion: {}", suggestion);
            }
        }
    }

    // Print warnings
    for warning in result.warnings {
        println!("⚠ Warning: {}", warning.message);
    }
}
```

### Format Auto-Detection

```rust
use schema_registry_validation::detect_format;

let schema = r#"{"$schema": "http://json-schema.org/draft-07/schema#"}"#;
let format = detect_format(schema)?;
println!("Detected format: {:?}", format); // JsonSchema
```

### Custom Configuration

```rust
use schema_registry_validation::{ValidationEngine, ValidationConfig};

let config = ValidationConfig::default()
    .with_fail_fast(true)              // Stop on first error
    .with_warnings(false)              // Suppress warnings
    .with_max_size(500_000);           // 500KB max schema size

let engine = ValidationEngine::with_config(config);
```

### Custom Validation Rules

```rust
use schema_registry_validation::{ValidationEngine, ValidationRule, ValidationError, SchemaFormat, Severity};
use std::sync::Arc;

struct CustomRule;

impl ValidationRule for CustomRule {
    fn name(&self) -> &str {
        "custom-company-rule"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn validate(&self, schema: &str, format: SchemaFormat) -> anyhow::Result<Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Custom validation logic
        if !schema.contains("company_metadata") {
            errors.push(
                ValidationError::new(
                    self.name(),
                    "Schema should include company_metadata for internal tracking"
                )
            );
        }

        Ok(errors)
    }
}

let mut engine = ValidationEngine::new();
engine.add_rule(Arc::new(CustomRule));
```

## Format-Specific Validators

### JSON Schema

```rust
use schema_registry_validation::JsonSchemaValidator;

let validator = JsonSchemaValidator::new_draft_7();

// Validate schema
let result = validator.validate(schema)?;

// Validate instance against schema
let instance = r#"{"name": "John", "age": 30}"#;
let result = validator.validate_instance(schema, instance)?;
```

### Apache Avro

```rust
use schema_registry_validation::AvroValidator;

let validator = AvroValidator::new();

let schema = r#"{
    "type": "record",
    "name": "User",
    "namespace": "com.example",
    "fields": [
        {"name": "id", "type": "long"},
        {"name": "username", "type": "string"}
    ]
}"#;

let result = validator.validate(schema)?;
```

### Protocol Buffers

```rust
use schema_registry_validation::ProtobufValidator;

let validator = ProtobufValidator::new();

let schema = r#"
syntax = "proto3";

package example;

message User {
  int64 id = 1;
  string username = 2;
}
"#;

let result = validator.validate(schema)?;
```

## Validation Results

### Error Details

```rust
for error in result.errors {
    println!("Rule: {}", error.rule);
    println!("Message: {}", error.message);
    println!("Severity: {:?}", error.severity);

    if let Some(location) = error.location {
        println!("Location: {}", location);
    }

    if let Some(line) = error.line {
        println!("Line: {}, Column: {}", line, error.column.unwrap_or(0));
    }

    if let Some(suggestion) = error.suggestion {
        println!("Suggestion: {}", suggestion);
    }

    for (key, value) in &error.context {
        println!("  {}: {}", key, value);
    }
}
```

### Metrics

```rust
println!("Validation Duration: {:?}", result.metrics.duration);
println!("Rules Applied: {}", result.metrics.rules_applied);
println!("Fields Validated: {}", result.metrics.fields_validated);
println!("Schema Size: {} bytes", result.metrics.schema_size_bytes);
println!("Max Recursion Depth: {}", result.metrics.max_recursion_depth);
```

## Performance Benchmarks

Run benchmarks with:

```bash
cargo bench --package schema-registry-validation
```

Expected results (on modern hardware):

| Operation | p50 | p95 | p99 |
|-----------|-----|-----|-----|
| JSON Schema (simple) | <5ms | <10ms | <15ms |
| JSON Schema (complex) | <15ms | <30ms | <45ms |
| Avro Validation | <3ms | <8ms | <12ms |
| Protobuf Validation | <2ms | <5ms | <8ms |
| Format Detection | <1ms | <2ms | <3ms |

## Testing

Run tests with:

```bash
cargo test --package schema-registry-validation
```

Test coverage target: **>90%**

## Architecture

```
ValidationEngine
├── FormatDetection
│   ├── JSON Schema detection
│   ├── Avro detection
│   └── Protobuf detection
│
├── Validators
│   ├── JsonSchemaValidator
│   ├── AvroValidator
│   └── ProtobufValidator
│
└── ValidationPipeline
    ├── 1. Structural Validation
    ├── 2. Type Validation
    ├── 3. Semantic Validation
    ├── 4. Compatibility Validation
    ├── 5. Security Validation
    ├── 6. Performance Validation
    └── 7. Custom Rule Validation
```

## Integration with LLM Schema Registry

This validation engine is designed to integrate seamlessly with other LLM Schema Registry components:

- **schema-registry-core**: Provides core types and utilities
- **schema-registry-api**: Exposes validation via REST/gRPC APIs
- **schema-registry-compatibility**: Uses this engine for compatibility checks
- **schema-registry-storage**: Validates schemas before persistence

## Error Codes

| Code | Description |
|------|-------------|
| `schema-size` | Schema exceeds size limits |
| `structural-validity` | Invalid syntax for the format |
| `type-validation` | Invalid or unsupported types |
| `semantic-validation` | Logical inconsistency |
| `security-check` | Security concern detected |
| `security-complexity` | Schema too complex (DoS risk) |
| `performance-validation` | Performance concern |

## Best Practices

### 1. Always Include Descriptions

```json
{
  "type": "object",
  "description": "Clear description for LLM understanding",
  "properties": {
    "field": {
      "type": "string",
      "description": "Clear field description"
    }
  }
}
```

### 2. Provide Examples

```json
{
  "type": "string",
  "description": "Email address",
  "format": "email",
  "examples": ["user@example.com"]
}
```

### 3. Use Semantic Versioning

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://example.com/schemas/user/v1.0.0",
  "version": "1.0.0"
}
```

### 4. Keep Schemas Simple

- Avoid deep nesting (>10 levels)
- Limit schema size (<1MB)
- Use references for reusable components

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Apache-2.0 - See [LICENSE](../../LICENSE) for details.

## Support

- GitHub Issues: https://github.com/llm-schema-registry/issues
- Documentation: https://docs.llm-schema-registry.dev
