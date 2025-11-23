// Package schema_registry provides a production-ready, enterprise-grade Go SDK
// for the LLM Schema Registry.
//
// # Features
//
//   - Context support: All operations accept context.Context for cancellation and timeouts
//   - Generics: Type-safe operations using Go generics (Go 1.18+)
//   - Thread-safety: All operations are safe for concurrent use
//   - Automatic retries: Configurable exponential backoff with jitter
//   - Smart caching: Thread-safe LRU cache with TTL support
//   - Comprehensive error handling: Detailed error types with proper error wrapping
//   - Production-ready: Battle-tested patterns and enterprise-grade code quality
//
// # Basic Usage
//
// Create a client:
//
//	client, err := schema_registry.NewClient(schema_registry.ClientConfig{
//	    BaseURL: "http://localhost:8080",
//	    APIKey:  "your-api-key",
//	})
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer client.Close()
//
// Register a schema:
//
//	schema := &schema_registry.Schema{
//	    Namespace: "telemetry",
//	    Name:      "InferenceEvent",
//	    Version:   "1.0.0",
//	    Format:    schema_registry.SchemaFormatJSONSchema,
//	    Content:   `{"type": "object", "properties": {"model": {"type": "string"}}}`,
//	}
//
//	ctx := context.Background()
//	result, err := client.RegisterSchema(ctx, schema)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("Schema ID: %s\n", result.SchemaID)
//
// Retrieve a schema:
//
//	schema, err := client.GetSchema(ctx, schemaID)
//	if err != nil {
//	    var notFound *schema_registry.SchemaNotFoundError
//	    if errors.As(err, &notFound) {
//	        fmt.Println("Schema not found:", notFound.SchemaID)
//	    } else {
//	        log.Fatal(err)
//	    }
//	}
//
// Validate data:
//
//	data := map[string]interface{}{
//	    "model": "gpt-4",
//	}
//	result, err := client.ValidateData(ctx, schemaID, data)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	if !result.IsValid {
//	    fmt.Println("Validation errors:", result.Errors)
//	}
//
// Check compatibility:
//
//	newSchema := `{"type": "object", "properties": {"model": {"type": "string"}, "version": {"type": "number"}}}`
//	result, err := client.CheckCompatibility(ctx, schemaID, newSchema, schema_registry.CompatibilityBackward)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	if !result.IsCompatible {
//	    fmt.Println("Incompatibilities:", result.Incompatibilities)
//	}
//
// # Context and Cancellation
//
// All client methods accept a context.Context parameter for cancellation and timeouts:
//
//	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
//	defer cancel()
//
//	schema, err := client.GetSchema(ctx, schemaID)
//	if err != nil {
//	    if errors.Is(err, context.DeadlineExceeded) {
//	        fmt.Println("Request timed out")
//	    }
//	}
//
// # Error Handling
//
// The SDK provides detailed error types for proper error handling:
//
//	err := client.RegisterSchema(ctx, schema)
//	if err != nil {
//	    var validationErr *schema_registry.ValidationError
//	    var incompatibleErr *schema_registry.IncompatibleSchemaError
//	    var rateLimitErr *schema_registry.RateLimitError
//
//	    switch {
//	    case errors.As(err, &validationErr):
//	        fmt.Println("Validation failed:", validationErr.Errors)
//	    case errors.As(err, &incompatibleErr):
//	        fmt.Println("Incompatible:", incompatibleErr.Incompatibilities)
//	    case errors.As(err, &rateLimitErr):
//	        if rateLimitErr.RetryAfter != nil {
//	            fmt.Printf("Rate limited, retry after %d seconds\n", *rateLimitErr.RetryAfter)
//	        }
//	    default:
//	        log.Fatal(err)
//	    }
//	}
//
// # Configuration
//
// Customize client behavior with ClientConfig:
//
//	client, err := schema_registry.NewClient(schema_registry.ClientConfig{
//	    BaseURL:      "http://localhost:8080",
//	    APIKey:       "your-api-key",
//	    Timeout:      30 * time.Second,
//	    CacheTTL:     5 * time.Minute,
//	    CacheMaxSize: 1000,
//	    RetryConfig: &schema_registry.RetryConfig{
//	        MaxAttempts:       3,
//	        InitialBackoff:    500 * time.Millisecond,
//	        MaxBackoff:        10 * time.Second,
//	        BackoffMultiplier: 2.0,
//	        Jitter:            true,
//	    },
//	})
//
// # Thread Safety
//
// All client methods are safe for concurrent use across multiple goroutines:
//
//	var wg sync.WaitGroup
//	for i := 0; i < 10; i++ {
//	    wg.Add(1)
//	    go func(id int) {
//	        defer wg.Done()
//	        schema, err := client.GetSchema(ctx, schemaID)
//	        if err != nil {
//	            log.Printf("Worker %d failed: %v", id, err)
//	            return
//	        }
//	        log.Printf("Worker %d got schema: %s", id, schema.Name)
//	    }(i)
//	}
//	wg.Wait()
//
// # Caching
//
// The SDK includes a thread-safe LRU cache with TTL support:
//
//	// Get cache statistics
//	stats := client.CacheStats()
//	fmt.Printf("Cache hits: %d, misses: %d, evictions: %d\n",
//	    stats.Hits, stats.Misses, stats.Evictions)
//
//	// Clear cache
//	client.ClearCache()
//
// # Retry Logic
//
// Automatic retries with exponential backoff for transient errors:
//
//	// Customize retry behavior
//	retryConfig := schema_registry.RetryConfig{
//	    MaxAttempts:       5,
//	    InitialBackoff:    1 * time.Second,
//	    MaxBackoff:        30 * time.Second,
//	    BackoffMultiplier: 2.0,
//	    Jitter:            true,
//	    RetryableFunc: func(err error) bool {
//	        return schema_registry.IsRetryable(err)
//	    },
//	}
//
// # Production Best Practices
//
//   - Always use context.Context with appropriate timeouts
//   - Always defer client.Close() to release resources
//   - Use proper error handling with errors.Is() and errors.As()
//   - Monitor cache statistics for optimal cache sizing
//   - Configure retry behavior based on your SLA requirements
//   - Use structured logging for production debugging
//   - Enable metrics and monitoring for production deployments
//
// # Advanced Features
//
// Search schemas:
//
//	results, err := client.SearchSchemas(ctx, "telemetry", 10, 0)
//	for _, result := range results {
//	    fmt.Printf("Found: %s.%s v%s (score: %.2f)\n",
//	        result.Namespace, result.Name, result.Version, result.Score)
//	}
//
// List versions:
//
//	versions, err := client.ListVersions(ctx, "telemetry", "InferenceEvent")
//	for _, v := range versions {
//	    fmt.Printf("Version: %s (ID: %s, Created: %s)\n",
//	        v.Version, v.SchemaID, v.CreatedAt)
//	}
//
// Health check:
//
//	health, err := client.HealthCheck(ctx)
//	if err != nil {
//	    log.Fatal("Service unhealthy:", err)
//	}
//	fmt.Printf("Status: %s, Version: %s\n", health.Status, health.Version)
package schema_registry
