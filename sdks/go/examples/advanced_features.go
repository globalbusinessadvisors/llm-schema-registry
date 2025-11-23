package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"sync"
	"time"

	schema_registry "github.com/llm-schema-registry/go-sdk"
)

func main() {
	// Advanced Feature 1: Custom retry configuration
	fmt.Println("=== Advanced Feature 1: Custom Retry Configuration ===")
	customRetryConfig := &schema_registry.RetryConfig{
		MaxAttempts:       5,
		InitialBackoff:    1 * time.Second,
		MaxBackoff:        30 * time.Second,
		BackoffMultiplier: 2.0,
		Jitter:            true,
		RetryableFunc:     schema_registry.IsRetryable,
	}

	client, err := schema_registry.NewClient(schema_registry.ClientConfig{
		BaseURL:      "http://localhost:8080",
		APIKey:       "your-api-key-here",
		Timeout:      30 * time.Second,
		CacheTTL:     10 * time.Minute,
		CacheMaxSize: 2000,
		RetryConfig:  customRetryConfig,
		EnableCache:  true,
	})
	if err != nil {
		log.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()

	fmt.Println("✓ Client created with custom retry configuration\n")

	// Advanced Feature 2: Comprehensive error handling
	fmt.Println("=== Advanced Feature 2: Error Handling ===")
	ctx := context.Background()

	// Try to get a non-existent schema
	_, err = client.GetSchema(ctx, "non-existent-id")
	if err != nil {
		var notFoundErr *schema_registry.SchemaNotFoundError
		var authErr *schema_registry.SchemaRegistryError
		var rateLimitErr *schema_registry.RateLimitError

		switch {
		case errors.As(err, &notFoundErr):
			fmt.Printf("✓ Correctly handled SchemaNotFoundError: %s\n", notFoundErr.SchemaID)
		case errors.As(err, &rateLimitErr):
			fmt.Printf("Rate limited")
			if rateLimitErr.RetryAfter != nil {
				fmt.Printf(", retry after %d seconds\n", *rateLimitErr.RetryAfter)
			}
		case errors.As(err, &authErr):
			fmt.Printf("Authentication/Authorization error: %v\n", authErr)
		case errors.Is(err, context.DeadlineExceeded):
			fmt.Println("Request timed out")
		case errors.Is(err, context.Canceled):
			fmt.Println("Request canceled")
		default:
			fmt.Printf("Other error: %v\n", err)
		}
	}
	fmt.Println()

	// Advanced Feature 3: Context cancellation
	fmt.Println("=== Advanced Feature 3: Context Cancellation ===")
	cancelCtx, cancel := context.WithCancel(context.Background())

	// Cancel immediately to demonstrate cancellation handling
	cancel()

	_, err = client.GetSchema(cancelCtx, "some-id")
	if err != nil {
		if errors.Is(err, context.Canceled) {
			fmt.Println("✓ Request correctly canceled via context\n")
		}
	}

	// Advanced Feature 4: Concurrent operations (thread-safety)
	fmt.Println("=== Advanced Feature 4: Concurrent Operations ===")

	// Register a schema first
	schema := &schema_registry.Schema{
		Namespace: "performance",
		Name:      "ConcurrentTest",
		Version:   "1.0.0",
		Format:    schema_registry.SchemaFormatJSONSchema,
		Content:   `{"type": "object", "properties": {"id": {"type": "number"}}}`,
	}

	result, err := client.RegisterSchema(context.Background(), schema)
	if err != nil {
		log.Fatalf("Failed to register schema: %v", err)
	}
	schemaID := result.SchemaID

	// Perform concurrent reads (demonstrates thread-safety)
	var wg sync.WaitGroup
	numWorkers := 50
	successCount := 0
	var mu sync.Mutex

	for i := 0; i < numWorkers; i++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			ctx := context.Background()
			_, err := client.GetSchema(ctx, schemaID)
			if err == nil {
				mu.Lock()
				successCount++
				mu.Unlock()
			}
		}(i)
	}

	wg.Wait()
	fmt.Printf("✓ Executed %d concurrent requests, %d successful\n", numWorkers, successCount)
	fmt.Println()

	// Advanced Feature 5: Cache management
	fmt.Println("=== Advanced Feature 5: Cache Management ===")

	// Warm up cache
	for i := 0; i < 5; i++ {
		_, _ = client.GetSchema(context.Background(), schemaID)
	}

	stats := client.CacheStats()
	fmt.Printf("Cache statistics after warming:\n")
	fmt.Printf("  Hits: %d, Misses: %d, Evictions: %d\n", stats.Hits, stats.Misses, stats.Evictions)

	// Clear cache
	client.ClearCache()
	fmt.Println("✓ Cache cleared\n")

	// Advanced Feature 6: Timeout handling
	fmt.Println("=== Advanced Feature 6: Timeout Handling ===")
	shortCtx, shortCancel := context.WithTimeout(context.Background(), 1*time.Nanosecond)
	defer shortCancel()

	time.Sleep(10 * time.Millisecond) // Ensure timeout

	_, err = client.GetSchema(shortCtx, schemaID)
	if err != nil {
		if errors.Is(err, context.DeadlineExceeded) {
			fmt.Println("✓ Timeout correctly handled\n")
		}
	}

	// Advanced Feature 7: Validation with error details
	fmt.Println("=== Advanced Feature 7: Detailed Validation Errors ===")

	invalidData := map[string]interface{}{
		"wrong_field": "invalid",
	}

	validateResult, err := client.ValidateData(context.Background(), schemaID, invalidData)
	if err != nil {
		fmt.Printf("Validation request error: %v\n", err)
	} else if !validateResult.IsValid {
		fmt.Printf("✓ Validation failed as expected:\n")
		for _, errMsg := range validateResult.Errors {
			fmt.Printf("  - %s\n", errMsg)
		}
	}
	fmt.Println()

	// Advanced Feature 8: Compatibility checking modes
	fmt.Println("=== Advanced Feature 8: Compatibility Modes ===")

	modes := []schema_registry.CompatibilityMode{
		schema_registry.CompatibilityBackward,
		schema_registry.CompatibilityForward,
		schema_registry.CompatibilityFull,
	}

	newSchema := `{"type": "object", "properties": {"id": {"type": "string"}}}`

	for _, mode := range modes {
		result, err := client.CheckCompatibility(context.Background(), schemaID, newSchema, mode)
		if err != nil {
			fmt.Printf("  %s: Error - %v\n", mode, err)
		} else {
			fmt.Printf("  %s: Compatible=%v\n", mode, result.IsCompatible)
		}
	}
	fmt.Println()

	// Advanced Feature 9: Resource cleanup
	fmt.Println("=== Advanced Feature 9: Resource Cleanup ===")
	if err := client.Close(); err != nil {
		log.Printf("Error closing client: %v", err)
	}
	fmt.Println("✓ Client resources cleaned up successfully\n")

	fmt.Println("=== All advanced features demonstrated successfully! ===")
}
