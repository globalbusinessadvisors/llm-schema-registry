# LLM Schema Registry - Compatibility Checker

The compatibility checker implements the 7-mode compatibility checking system as specified in [PSEUDOCODE.md § 1.5](../../plans/PSEUDOCODE.md#15-compatibility-checking-algorithm).

## Features

- **7 Compatibility Modes**: BACKWARD, FORWARD, FULL, BACKWARD_TRANSITIVE, FORWARD_TRANSITIVE, FULL_TRANSITIVE, NONE
- **3 Schema Formats**: JSON Schema, Apache Avro, Protocol Buffers
- **Performance**: p95 latency < 25ms (target)
- **Caching**: Built-in compatibility matrix caching for repeated checks
- **Dependency Analysis**: Track schema dependencies and calculate impact radius
- **Breaking Change Detection**: Comprehensive violation detection

## Compatibility Modes

### BACKWARD
New schema can read old data. This is the most common mode for evolving schemas.

**Allowed Changes:**
- Add optional fields
- Remove optional fields with defaults
- Relax constraints

**Breaking Changes:**
- Remove required fields
- Add required fields without defaults
- Change field types
- Tighten constraints

### FORWARD
Old schema can read new data. Useful when consumers lag behind producers.

**Allowed Changes:**
- Remove optional fields
- Add optional fields with defaults
- Relax constraints

**Breaking Changes:**
- Add required fields
- Remove required fields without defaults
- Change field types

### FULL
Both BACKWARD and FORWARD compatible. Most restrictive mode.

**Allowed Changes:**
- Only changes that are both backward and forward compatible

### TRANSITIVE Modes
Check compatibility against ALL previous versions, not just the latest.

- **BACKWARD_TRANSITIVE**: Backward compatible with all versions
- **FORWARD_TRANSITIVE**: Forward compatible with all versions
- **FULL_TRANSITIVE**: Full compatible with all versions

### NONE
No compatibility checking. Use with caution!

## Usage Examples

### Basic Compatibility Check

```rust
use llm_schema_registry_compatibility::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create checker with default config
    let checker = CompatibilityChecker::new(
        CompatibilityCheckerConfig::default()
    );

    // Define old schema
    let old_schema = Schema {
        id: Uuid::new_v4(),
        name: "User".to_string(),
        namespace: "com.example".to_string(),
        version: SemanticVersion::new(1, 0, 0),
        format: SchemaFormat::JsonSchema,
        content: r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["name", "email"]
        }"#.to_string(),
        content_hash: Schema::calculate_hash("..."),
        description: "User schema".to_string(),
        compatibility_mode: CompatibilityMode::Backward,
        created_at: Utc::now(),
        metadata: Default::default(),
    };

    // Define new schema (added optional field)
    let new_schema = Schema {
        // ... same as old, but with:
        content: r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name", "email"]
        }"#.to_string(),
        version: SemanticVersion::new(1, 1, 0),
        // ...
    };

    // Check backward compatibility
    let result = checker.check_compatibility(
        &new_schema,
        &old_schema,
        CompatibilityMode::Backward,
    ).await?;

    if result.is_compatible {
        println!("✓ Schemas are backward compatible!");
        println!("Check duration: {}ms", result.check_duration_ms);
    } else {
        println!("✗ Breaking changes detected:");
        for violation in &result.violations {
            println!("  - {} at {}: {}",
                violation.violation_type,
                violation.field_path,
                violation.description
            );
        }
    }

    Ok(())
}
```

### Transitive Compatibility Check

```rust
// Check against all previous versions
let result = checker.check_compatibility_transitive(
    &new_schema,
    CompatibilityMode::BackwardTransitive,
    |namespace, name| {
        // Fetch all versions from storage
        fetch_all_schema_versions(namespace, name)
    },
).await?;

println!("Checked against {} versions", result.checked_versions.len());
println!("Impact radius: {} schemas", result.checked_versions.len());
```

### Using the Cache

```rust
// Enable caching for better performance
let config = CompatibilityCheckerConfig {
    enable_cache: true,
    max_cache_size: 10_000,
    cache_ttl_seconds: 3600, // 1 hour
    max_transitive_versions: 100,
    check_timeout_ms: 25,
};

let checker = CompatibilityChecker::new(config);

// First check - cache miss
let result1 = checker.check_compatibility(
    &new_schema,
    &old_schema,
    CompatibilityMode::Backward,
).await?;

// Second check - cache hit (much faster!)
let result2 = checker.check_compatibility(
    &new_schema,
    &old_schema,
    CompatibilityMode::Backward,
).await?;

// Get cache statistics
if let Some((hits, misses, hit_rate)) = checker.cache_stats() {
    println!("Cache stats: {} hits, {} misses, {:.2}% hit rate",
        hits, misses, hit_rate * 100.0);
}
```

### Dependency Graph Analysis

```rust
use llm_schema_registry_compatibility::dependency::*;

// Build dependency graph
let mut graph = DependencyGraph::new();

graph.add_dependency(SchemaDependency {
    schema_id: schema_a_id,
    version: SemanticVersion::new(1, 0, 0),
    depends_on_schema_id: schema_b_id,
    depends_on_version: SemanticVersion::new(1, 0, 0),
    dependency_type: DependencyType::Reference,
});

// Find all schemas that depend on this one
let dependents = graph.get_transitive_dependents(&schema_b_id);
println!("Breaking this schema will affect {} schemas", dependents.len());

// Detect circular dependencies
if graph.has_circular_dependency(&schema_a_id) {
    println!("⚠ Circular dependency detected!");
}

// Calculate impact of breaking changes
let impact = ImpactAnalysis::calculate(
    schema_id,
    breaking_violations,
    &graph,
);

println!("Impact Analysis:");
println!("  Direct dependents: {}", impact.direct_dependents.len());
println!("  Total impact radius: {}", impact.impact_radius);
println!("  Migration order: {:?}", impact.migration_order);

// Generate migration guide
let guide = impact.generate_migration_guide();
println!("{}", guide);
```

## Breaking Change Examples

### JSON Schema

#### Field Removal (BREAKING)
```json
// Old schema
{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "email": {"type": "string"}
  }
}

// New schema - BREAKING
{
  "type": "object",
  "properties": {
    "name": {"type": "string"}
    // "email" removed - BREAKING!
  }
}
```

#### Adding Required Field (BREAKING)
```json
// Old schema
{
  "type": "object",
  "properties": {
    "name": {"type": "string"}
  }
}

// New schema - BREAKING
{
  "type": "object",
  "properties": {
    "name": {"type": "string"},
    "email": {"type": "string"}
  },
  "required": ["email"]  // BREAKING!
}
```

#### Type Change (BREAKING)
```json
// Old: string
{"age": {"type": "string"}}

// New: integer - BREAKING!
{"age": {"type": "integer"}}
```

#### Constraint Tightening (BREAKING)
```json
// Old
{"age": {"type": "integer", "minimum": 0}}

// New - BREAKING!
{"age": {"type": "integer", "minimum": 18}}
```

### Apache Avro

#### Field Removal without Default (BREAKING)
```json
// Old
{
  "type": "record",
  "name": "User",
  "fields": [
    {"name": "name", "type": "string"},
    {"name": "email", "type": "string"}
  ]
}

// New - BREAKING
{
  "type": "record",
  "name": "User",
  "fields": [
    {"name": "name", "type": "string"}
    // "email" removed without default - BREAKING!
  ]
}
```

#### Type Promotion (COMPATIBLE)
```json
// Old
{"name": "age", "type": "int"}

// New - COMPATIBLE (int -> long promotion)
{"name": "age", "type": "long"}
```

### Protocol Buffers

#### Field Number Reuse (CRITICAL BREAKING)
```protobuf
// Old
message User {
  optional string name = 1;
  optional string email = 2;
}

// New - CRITICAL BREAKING!
message User {
  optional string name = 1;
  optional int32 email = 2;  // Reused number with different type!
}
```

#### Optional to Required (BREAKING)
```protobuf
// Old
message User {
  optional string name = 1;
}

// New - BREAKING!
message User {
  required string name = 1;
}
```

## Performance Benchmarks

Run benchmarks:
```bash
cd crates/compatibility-checker
cargo bench
```

### Expected Performance

| Operation | Target | Typical |
|-----------|--------|---------|
| Simple JSON Schema Check | < 10ms | ~3ms |
| Complex Schema (50 fields) | < 25ms | ~15ms |
| Avro Schema Check | < 10ms | ~4ms |
| Protobuf Schema Check | < 10ms | ~3ms |
| Cache Hit | < 1ms | ~0.1ms |
| Transitive Check (10 versions) | < 100ms | ~50ms |

## Testing

Run unit and integration tests:
```bash
cargo test
```

Run with coverage:
```bash
cargo tarpaulin --out Html
```

## API Reference

### `CompatibilityChecker`

Main compatibility checker interface.

#### Methods

- `new(config: CompatibilityCheckerConfig) -> Self`
- `check_compatibility(&self, new: &Schema, old: &Schema, mode: CompatibilityMode) -> Result<CompatibilityResult>`
- `check_compatibility_transitive<F>(&self, new: &Schema, mode: CompatibilityMode, fetch: F) -> Result<CompatibilityResult>`
- `cache_stats(&self) -> Option<(u64, u64, f64)>`

### `CompatibilityResult`

Result of a compatibility check.

#### Fields

- `is_compatible: bool` - Whether schemas are compatible
- `mode: CompatibilityMode` - Mode used for checking
- `violations: Vec<CompatibilityViolation>` - List of violations found
- `checked_versions: Vec<SemanticVersion>` - Versions checked against
- `check_duration_ms: u64` - Time taken for check
- `metadata: HashMap<String, Value>` - Additional metadata

### `CompatibilityViolation`

Represents a compatibility violation.

#### Fields

- `violation_type: ViolationType` - Type of violation
- `field_path: String` - Path to the problematic field
- `old_value: Option<Value>` - Old value
- `new_value: Option<Value>` - New value
- `severity: ViolationSeverity` - Severity (Breaking, Warning, Info)
- `description: String` - Human-readable description

## Integration with Schema Registry

```rust
// In your schema registration flow:
async fn register_schema(
    schema: Schema,
    checker: &CompatibilityChecker,
    storage: &dyn SchemaStorage,
) -> Result<Uuid, RegistrationError> {
    // Fetch previous version
    let previous = storage
        .get_latest_version(&schema.namespace, &schema.name)
        .await?;

    if let Some(prev) = previous {
        // Check compatibility
        let result = checker
            .check_compatibility(
                &schema,
                &prev,
                schema.compatibility_mode,
            )
            .await?;

        if !result.is_compatible {
            return Err(RegistrationError::IncompatibleSchema(
                result.violations
            ));
        }
    }

    // Store schema
    storage.store(schema).await
}
```

## Contributing

See the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Apache-2.0 - See [LICENSE](../../LICENSE) for details.
