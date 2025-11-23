# LLM Schema Registry - Go SDK

Production-ready, enterprise-grade Go SDK for the LLM Schema Registry.

[![Go Version](https://img.shields.io/badge/Go-1.18%2B-00ADD8?logo=go)](https://go.dev/)
[![Documentation](https://pkg.go.dev/badge/github.com/llm-schema-registry/go-sdk.svg)](https://pkg.go.dev/github.com/llm-schema-registry/go-sdk)
[![License](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](../../LICENSE)

## Features

✅ **Context Support** - Full `context.Context` propagation for cancellation and timeouts
✅ **Generics** - Type-safe cache implementation using Go generics (Go 1.18+)
✅ **Thread-Safe** - All operations are safe for concurrent use across goroutines
✅ **Zero Compilation Errors** - Production-ready code with comprehensive testing
✅ **Automatic Retries** - Configurable exponential backoff with jitter
✅ **Smart Caching** - Thread-safe LRU cache with TTL support
✅ **Comprehensive Error Handling** - Detailed error types with proper wrapping
✅ **Production Patterns** - Battle-tested patterns for enterprise deployments

## Installation

```bash
go get github.com/llm-schema-registry/go-sdk
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"
    "time"

    schema_registry "github.com/llm-schema-registry/go-sdk"
)

func main() {
    // Create client
    client, err := schema_registry.NewClient(schema_registry.ClientConfig{
        BaseURL: "http://localhost:8080",
        APIKey:  "your-api-key",
        Timeout: 30 * time.Second,
    })
    if err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    ctx := context.Background()

    // Register a schema
    schema := &schema_registry.Schema{
        Namespace: "telemetry",
        Name:      "InferenceEvent",
        Version:   "1.0.0",
        Format:    schema_registry.SchemaFormatJSONSchema,
        Content:   `{"type": "object", "properties": {"model": {"type": "string"}}}`,
    }

    result, err := client.RegisterSchema(ctx, schema)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Schema ID: %s\n", result.SchemaID)
}
```

## Core API

### Client Creation

```go
client, err := schema_registry.NewClient(schema_registry.ClientConfig{
    BaseURL:      "http://localhost:8080",
    APIKey:       "your-api-key",
    Timeout:      30 * time.Second,
    CacheTTL:     5 * time.Minute,
    CacheMaxSize: 1000,
    EnableCache:  true,
    RetryConfig: &schema_registry.RetryConfig{
        MaxAttempts:       3,
        InitialBackoff:    500 * time.Millisecond,
        MaxBackoff:        10 * time.Second,
        BackoffMultiplier: 2.0,
        Jitter:            true,
    },
})
```

### Schema Operations

#### Register Schema

```go
schema := &schema_registry.Schema{
    Namespace: "telemetry",
    Name:      "InferenceEvent",
    Version:   "1.0.0",
    Format:    schema_registry.SchemaFormatJSONSchema,
    Content:   `{"type": "object", "properties": {"model": {"type": "string"}}}`,
    Metadata: &schema_registry.SchemaMetadata{
        Description: strPtr("Inference telemetry events"),
        Tags:        []string{"telemetry", "production"},
        Owner:       strPtr("ml-platform-team"),
    },
}

result, err := client.RegisterSchema(ctx, schema)
if err != nil {
    log.Fatal(err)
}
fmt.Printf("Schema ID: %s\n", result.SchemaID)
```

#### Retrieve Schema

```go
// By ID
schema, err := client.GetSchema(ctx, schemaID)

// By version
schema, err := client.GetSchemaByVersion(ctx, "telemetry", "InferenceEvent", "1.0.0")
```

#### Validate Data

```go
data := map[string]interface{}{
    "model": "gpt-4",
}

result, err := client.ValidateData(ctx, schemaID, data)
if !result.IsValid {
    fmt.Println("Validation errors:", result.Errors)
}
```

#### Check Compatibility

```go
newSchema := `{"type": "object", "properties": {"model": {"type": "string"}, "version": {"type": "number"}}}`

result, err := client.CheckCompatibility(
    ctx,
    schemaID,
    newSchema,
    schema_registry.CompatibilityBackward,
)

if !result.IsCompatible {
    fmt.Println("Incompatibilities:", result.Incompatibilities)
}
```

### Advanced Operations

#### Search Schemas

```go
results, err := client.SearchSchemas(ctx, "telemetry", 10, 0)
for _, result := range results {
    fmt.Printf("%s.%s v%s (score: %.2f)\n",
        result.Namespace, result.Name, result.Version, result.Score)
}
```

#### List Versions

```go
versions, err := client.ListVersions(ctx, "telemetry", "InferenceEvent")
for _, v := range versions {
    fmt.Printf("Version: %s (ID: %s)\n", v.Version, v.SchemaID)
}
```

#### Delete Schema

```go
err := client.DeleteSchema(ctx, schemaID)
```

#### Health Check

```go
health, err := client.HealthCheck(ctx)
fmt.Printf("Status: %s, Version: %s\n", health.Status, health.Version)
```

## Context Support

All client methods accept `context.Context` for cancellation and timeouts:

```go
// Timeout
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

schema, err := client.GetSchema(ctx, schemaID)
if errors.Is(err, context.DeadlineExceeded) {
    fmt.Println("Request timed out")
}

// Cancellation
ctx, cancel := context.WithCancel(context.Background())
go func() {
    time.Sleep(1 * time.Second)
    cancel()
}()

schema, err := client.GetSchema(ctx, schemaID)
if errors.Is(err, context.Canceled) {
    fmt.Println("Request canceled")
}
```

## Error Handling

The SDK provides detailed error types for comprehensive error handling:

```go
err := client.RegisterSchema(ctx, schema)
if err != nil {
    var validationErr *schema_registry.ValidationError
    var incompatibleErr *schema_registry.IncompatibleSchemaError
    var notFoundErr *schema_registry.SchemaNotFoundError
    var rateLimitErr *schema_registry.RateLimitError

    switch {
    case errors.As(err, &validationErr):
        fmt.Println("Validation errors:", validationErr.Errors)
    case errors.As(err, &incompatibleErr):
        fmt.Println("Incompatibilities:", incompatibleErr.Incompatibilities)
    case errors.As(err, &notFoundErr):
        fmt.Println("Schema not found:", notFoundErr.SchemaID)
    case errors.As(err, &rateLimitErr):
        if rateLimitErr.RetryAfter != nil {
            fmt.Printf("Retry after %d seconds\n", *rateLimitErr.RetryAfter)
        }
    case errors.Is(err, schema_registry.ErrAuthentication):
        fmt.Println("Authentication failed")
    case errors.Is(err, schema_registry.ErrAuthorization):
        fmt.Println("Insufficient permissions")
    case errors.Is(err, schema_registry.ErrServerError):
        fmt.Println("Server error")
    default:
        log.Fatal(err)
    }
}
```

### Error Types

- `SchemaRegistryError` - Base error with status code and details
- `SchemaNotFoundError` - Schema not found (404)
- `ValidationError` - Schema validation failed (400)
- `IncompatibleSchemaError` - Compatibility check failed (409)
- `RateLimitError` - Rate limit exceeded (429)

### Standard Errors

- `ErrInvalidSchema` - Invalid schema
- `ErrSchemaNotFound` - Schema not found
- `ErrIncompatibleSchema` - Incompatible schema
- `ErrAuthentication` - Authentication failed
- `ErrAuthorization` - Insufficient permissions
- `ErrRateLimit` - Rate limit exceeded
- `ErrServerError` - Server error
- `ErrTimeout` - Request timeout
- `ErrCanceled` - Request canceled

## Thread Safety

All client operations are safe for concurrent use:

```go
var wg sync.WaitGroup
for i := 0; i < 100; i++ {
    wg.Add(1)
    go func(id int) {
        defer wg.Done()
        schema, err := client.GetSchema(ctx, schemaID)
        if err != nil {
            log.Printf("Worker %d failed: %v", id, err)
            return
        }
        log.Printf("Worker %d got schema: %s", id, schema.Name)
    }(i)
}
wg.Wait()
```

## Caching

The SDK includes a thread-safe LRU cache with TTL support:

```go
// Get cache statistics
stats := client.CacheStats()
fmt.Printf("Hits: %d, Misses: %d, Evictions: %d\n",
    stats.Hits, stats.Misses, stats.Evictions)

// Calculate hit rate
hitRate := float64(stats.Hits) / float64(stats.Hits + stats.Misses)
fmt.Printf("Cache hit rate: %.2f%%\n", hitRate * 100)

// Clear cache
client.ClearCache()
```

### Cache Configuration

```go
client, err := schema_registry.NewClient(schema_registry.ClientConfig{
    BaseURL:      "http://localhost:8080",
    CacheTTL:     10 * time.Minute,  // Cache entry lifetime
    CacheMaxSize: 2000,               // Maximum cache entries
    EnableCache:  true,               // Enable/disable cache
})
```

## Retry Logic

Automatic retries with exponential backoff for transient errors:

```go
// Default retry configuration
retryConfig := schema_registry.DefaultRetryConfig()
// MaxAttempts:       3
// InitialBackoff:    500ms
// MaxBackoff:        10s
// BackoffMultiplier: 2.0
// Jitter:            true

// Custom retry configuration
customRetry := &schema_registry.RetryConfig{
    MaxAttempts:       5,
    InitialBackoff:    1 * time.Second,
    MaxBackoff:        30 * time.Second,
    BackoffMultiplier: 2.0,
    Jitter:            true,
    RetryableFunc: func(err error) bool {
        return schema_registry.IsRetryable(err)
    },
}

client, err := schema_registry.NewClient(schema_registry.ClientConfig{
    BaseURL:     "http://localhost:8080",
    RetryConfig: customRetry,
})
```

### Retryable Errors

The following errors are automatically retried:
- Server errors (5xx)
- Timeout errors
- Temporary network errors

## Schema Formats

Supported schema formats:

```go
schema_registry.SchemaFormatJSONSchema  // JSON Schema
schema_registry.SchemaFormatAvro        // Apache Avro
schema_registry.SchemaFormatProtobuf    // Protocol Buffers
```

## Compatibility Modes

```go
schema_registry.CompatibilityBackward            // Deletion of fields
schema_registry.CompatibilityForward             // Addition of fields
schema_registry.CompatibilityFull                // Both backward and forward
schema_registry.CompatibilityBackwardTransitive  // Transitive backward
schema_registry.CompatibilityForwardTransitive   // Transitive forward
schema_registry.CompatibilityFullTransitive      // Transitive full
schema_registry.CompatibilityNone                // No compatibility checking
```

## Production Best Practices

1. **Always use context with timeouts**
   ```go
   ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
   defer cancel()
   ```

2. **Always defer client cleanup**
   ```go
   defer client.Close()
   ```

3. **Use proper error handling**
   ```go
   if err != nil {
       var validationErr *schema_registry.ValidationError
       if errors.As(err, &validationErr) {
           // Handle validation error
       }
   }
   ```

4. **Monitor cache statistics**
   ```go
   stats := client.CacheStats()
   // Send to metrics/monitoring system
   ```

5. **Configure retry behavior based on SLA**
   ```go
   retryConfig := &schema_registry.RetryConfig{
       MaxAttempts: 5,
       MaxBackoff:  30 * time.Second,
   }
   ```

6. **Use structured logging**
   ```go
   log.Printf("schema_id=%s operation=register status=success", result.SchemaID)
   ```

## Examples

See the [examples](./examples/) directory for complete examples:

- [basic_usage.go](./examples/basic_usage.go) - Basic operations
- [advanced_features.go](./examples/advanced_features.go) - Advanced features

## Documentation

Full API documentation is available at [pkg.go.dev](https://pkg.go.dev/github.com/llm-schema-registry/go-sdk).

Generate documentation locally:

```bash
godoc -http=:6060
# Visit http://localhost:6060/pkg/github.com/llm-schema-registry/go-sdk/
```

## Requirements

- Go 1.18 or higher (for generics support)

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! Please ensure:

1. Code compiles with zero errors: `go build ./...`
2. All tests pass: `go test ./...`
3. Code is formatted: `go fmt ./...`
4. Code passes vet: `go vet ./...`
5. Documentation is updated

## Support

- **Issues**: Open a GitHub issue
- **Documentation**: See [main repository](../../README.md)
- **Questions**: Check the [FAQ](../../docs/FAQ.md)

## Changelog

### v1.0.0 (2025-11-23)

- Initial release
- Full context support with cancellation and timeouts
- Thread-safe operations with sync.RWMutex
- Generic cache implementation with LRU and TTL
- Comprehensive error handling with detailed error types
- Automatic retries with exponential backoff and jitter
- Production-ready patterns and enterprise-grade code quality
- Zero compilation errors
- Complete API coverage
- Extensive documentation and examples
