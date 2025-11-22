# Testing Guide - LLM Schema Registry

This guide provides comprehensive information on testing the LLM Schema Registry.

## Quick Start

```bash
# Run all tests
./scripts/run-tests.sh all

# Run specific test type
./scripts/run-tests.sh unit
./scripts/run-tests.sh integration
./scripts/run-tests.sh e2e
./scripts/run-tests.sh property

# Run with coverage
./scripts/run-tests.sh all coverage
```

## Test Organization

### Directory Structure

```
llm-schema-registry/
├── tests/
│   ├── integration/          # Integration tests (100+)
│   │   ├── mod.rs
│   │   ├── test_environment.rs
│   │   ├── database_tests.rs
│   │   ├── redis_tests.rs
│   │   ├── s3_tests.rs
│   │   ├── multi_tier_storage_tests.rs
│   │   └── api_integration_tests.rs
│   ├── e2e/                  # End-to-end tests (50+)
│   │   ├── mod.rs
│   │   ├── schema_lifecycle_tests.rs
│   │   ├── validation_workflow_tests.rs
│   │   ├── compatibility_workflow_tests.rs
│   │   ├── multi_version_tests.rs
│   │   └── error_handling_tests.rs
│   ├── property/             # Property-based tests (30+)
│   │   ├── mod.rs
│   │   ├── schema_properties.rs
│   │   ├── compatibility_properties.rs
│   │   └── validation_properties.rs
│   ├── load/                 # Load tests (k6)
│   │   ├── basic_load.js
│   │   ├── spike_test.js
│   │   ├── stress_test.js
│   │   └── soak_test.js
│   └── chaos/                # Chaos engineering
│       ├── pod-failure.yaml
│       ├── network-latency.yaml
│       ├── network-partition.yaml
│       ├── resource-stress.yaml
│       └── database-failure.yaml
├── crates/*/tests/           # Unit tests (400+)
└── scripts/
    └── run-tests.sh          # Test runner script
```

## Test Types

### 1. Unit Tests (400+)

Located in each crate under `crates/*/tests/`

**Purpose:** Test individual components in isolation

**Running:**
```bash
cargo test --lib --bins --all-features
```

**Coverage Areas:**
- Validation engine (JSON Schema, Avro, Protobuf)
- Compatibility checker (all modes)
- Storage layer (mocked)
- API handlers (mocked)
- Security module
- State machine transitions

### 2. Integration Tests (100+)

Located in `tests/integration/`

**Purpose:** Test interactions between components with real services

**Prerequisites:**
- Docker (for testcontainers)
- PostgreSQL, Redis, LocalStack containers

**Running:**
```bash
cargo test --test integration --all-features
```

**Coverage Areas:**
- Database operations (CRUD, transactions, migrations)
- Redis caching (set, get, invalidation)
- S3 operations (upload, download, lifecycle)
- Multi-tier storage integration

**Key Features:**
- Automatic container management via testcontainers
- Isolated test environments
- Data cleanup between tests

### 3. End-to-End Tests (50+)

Located in `tests/e2e/`

**Purpose:** Test complete user workflows

**Running:**
```bash
cargo test --test e2e --all-features
```

**Coverage Areas:**
- Schema registration flow
- Validation workflow
- Compatibility checking flow
- Schema evolution scenarios
- Error handling paths

### 4. Property-Based Tests (30+)

Located in `tests/property/`

**Purpose:** Test algorithmic correctness with generated inputs

**Running:**
```bash
cargo test --test property --all-features
```

**Coverage Areas:**
- Schema serialization/deserialization
- Compatibility checking reflexivity
- State machine invariants
- Hash calculation determinism
- Validation logic correctness

### 5. Load Tests (4 scenarios)

Located in `tests/load/`

**Purpose:** Validate performance under load

**Prerequisites:**
- k6 installed
- Services running

**Running:**
```bash
# Install k6
brew install k6  # macOS
# or from https://k6.io/

# Start services
docker-compose -f docker/docker-compose.test.yml up -d

# Run tests
k6 run tests/load/basic_load.js
k6 run tests/load/spike_test.js
k6 run tests/load/stress_test.js
k6 run tests/load/soak_test.js
```

**Test Scenarios:**

| Test | Duration | Target | Purpose |
|------|----------|--------|---------|
| basic_load.js | 25 min | 10K req/sec | Sustained load |
| spike_test.js | 10 min | Sudden surge | Traffic spike handling |
| stress_test.js | 60 min | Progressive | Find breaking point |
| soak_test.js | 2 hours | 200 users | Memory leak detection |

### 6. Chaos Tests (5 scenarios)

Located in `tests/chaos/`

**Purpose:** Test resilience under failure conditions

**Prerequisites:**
- Kubernetes cluster
- Chaos Mesh installed

**Running:**
```bash
# Install Chaos Mesh
kubectl create ns chaos-mesh
helm install chaos-mesh chaos-mesh/chaos-mesh -n=chaos-mesh

# Apply scenarios
kubectl apply -f tests/chaos/pod-failure.yaml
kubectl apply -f tests/chaos/network-latency.yaml
kubectl apply -f tests/chaos/resource-stress.yaml
```

**Scenarios:**
- Pod failures
- Network latency
- Network partitions
- Resource exhaustion
- Database failures

## Code Coverage

### Running Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --config tarpaulin.toml --engine llvm

# View HTML report
open target/coverage/index.html
```

### Coverage Targets

| Crate | Target |
|-------|--------|
| schema-registry-core | >90% |
| schema-registry-validation | >90% |
| schema-registry-compatibility | >90% |
| schema-registry-storage | >85% |
| schema-registry-api | >85% |
| schema-registry-security | >90% |
| schema-registry-observability | >80% |
| **Overall** | **>85%** |

## CI/CD Integration

### GitHub Actions Workflows

**test.yml** - Runs on every push/PR
- Unit tests
- Integration tests
- Property tests
- Code coverage
- Linting
- Security audit
- Benchmarks

**load-tests.yml** - Weekly schedule + manual
- Basic load test
- Spike test
- Stress test
- Soak test (scheduled only)

### Running in CI

Tests run automatically on:
- Push to main/develop
- Pull requests
- Weekly schedule (load tests)

Manual triggers:
- Workflow dispatch (GitHub UI)
- API call

## Best Practices

### Writing Tests

1. **Use descriptive test names**
   ```rust
   #[test]
   fn test_schema_registration_with_valid_json_schema_succeeds() {
       // ...
   }
   ```

2. **Follow AAA pattern**
   ```rust
   // Arrange
   let schema = create_test_schema();

   // Act
   let result = register_schema(schema).await;

   // Assert
   assert!(result.is_ok());
   ```

3. **Clean up test data**
   ```rust
   #[tokio::test]
   async fn test_something() {
       let env = TestEnvironment::new().await.unwrap();
       env.reset().await.unwrap(); // Clean slate

       // Test logic...
   }
   ```

4. **Test error cases**
   ```rust
   #[test]
   fn test_invalid_schema_returns_error() {
       let invalid_schema = "not a schema";
       let result = validate_schema(invalid_schema);
       assert!(result.is_err());
   }
   ```

### Test Performance

1. **Keep unit tests fast** (<1s each)
2. **Parallelize when possible**
3. **Use test fixtures** for common data
4. **Mock external dependencies** in unit tests
5. **Use real services** in integration tests

### Test Maintenance

1. **Review failing tests** immediately
2. **Update tests** with code changes
3. **Refactor flaky tests**
4. **Document complex test setups**
5. **Keep test dependencies** up to date

## Troubleshooting

### Common Issues

1. **Docker not running**
   ```
   Error: Cannot connect to Docker daemon
   Solution: Start Docker Desktop
   ```

2. **Port conflicts**
   ```
   Error: Port 5432 already in use
   Solution: Stop conflicting service or change port
   ```

3. **Test timeouts**
   ```
   Error: Test exceeded timeout
   Solution: Increase timeout or optimize test
   ```

4. **Coverage below threshold**
   ```
   Error: Coverage 82% < 85%
   Solution: Add tests for uncovered code
   ```

### Debug Commands

```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run tests in single thread
cargo test -- --test-threads=1

# Show test output
cargo test -- --show-output

# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Verbose logging
RUST_LOG=debug cargo test
```

## Performance Benchmarks

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --all-features

# Run specific benchmark
cargo bench validation

# Compare with baseline
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

### Benchmark Locations

- `crates/schema-registry-validation/benches/`
- `crates/compatibility-checker/benches/`
- `crates/storage/benches/`

## Resources

- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Testcontainers Rust](https://github.com/testcontainers/testcontainers-rs)
- [k6 Documentation](https://k6.io/docs/)
- [Chaos Mesh Documentation](https://chaos-mesh.org/docs/)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [proptest](https://github.com/proptest-rs/proptest)

## Support

For questions or issues with testing:
- Create an issue in the repository
- Check existing test examples
- Review CI/CD logs
- Consult TEST-REPORT.md for detailed metrics
