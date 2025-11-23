package main

import (
	"context"
	"fmt"
	"log"
	"time"

	schema_registry "github.com/llm-schema-registry/go-sdk"
)

func main() {
	// Create a new client with default configuration
	client, err := schema_registry.NewClient(schema_registry.ClientConfig{
		BaseURL: "http://localhost:8080",
		APIKey:  "your-api-key-here",
		Timeout: 30 * time.Second,
	})
	if err != nil {
		log.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()

	// Create a context with timeout
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Example 1: Register a new schema
	fmt.Println("=== Example 1: Register Schema ===")
	schema := &schema_registry.Schema{
		Namespace: "telemetry",
		Name:      "InferenceEvent",
		Version:   "1.0.0",
		Format:    schema_registry.SchemaFormatJSONSchema,
		Content: `{
			"type": "object",
			"properties": {
				"model": {"type": "string"},
				"timestamp": {"type": "number"},
				"latency_ms": {"type": "number"}
			},
			"required": ["model", "timestamp", "latency_ms"]
		}`,
		Metadata: &schema_registry.SchemaMetadata{
			Description: strPtr("Schema for inference telemetry events"),
			Tags:        []string{"telemetry", "inference", "production"},
			Owner:       strPtr("ml-platform-team"),
		},
	}

	result, err := client.RegisterSchema(ctx, schema)
	if err != nil {
		log.Fatalf("Failed to register schema: %v", err)
	}
	fmt.Printf("✓ Schema registered successfully\n")
	fmt.Printf("  Schema ID: %s\n", result.SchemaID)
	fmt.Printf("  Version: %s\n", result.Version)
	fmt.Printf("  Created At: %s\n\n", result.CreatedAt)

	schemaID := result.SchemaID

	// Example 2: Retrieve a schema by ID
	fmt.Println("=== Example 2: Retrieve Schema by ID ===")
	retrievedSchema, err := client.GetSchema(ctx, schemaID)
	if err != nil {
		log.Fatalf("Failed to get schema: %v", err)
	}
	fmt.Printf("✓ Schema retrieved successfully\n")
	fmt.Printf("  Name: %s.%s\n", retrievedSchema.Namespace, retrievedSchema.Name)
	fmt.Printf("  Version: %s\n", retrievedSchema.Version)
	fmt.Printf("  Format: %s\n\n", retrievedSchema.Format)

	// Example 3: Retrieve schema by version
	fmt.Println("=== Example 3: Retrieve Schema by Version ===")
	schemaByVersion, err := client.GetSchemaByVersion(ctx, "telemetry", "InferenceEvent", "1.0.0")
	if err != nil {
		log.Fatalf("Failed to get schema by version: %v", err)
	}
	fmt.Printf("✓ Schema retrieved by version\n")
	fmt.Printf("  Schema ID: %s\n\n", schemaByVersion.SchemaID)

	// Example 4: Validate data against schema
	fmt.Println("=== Example 4: Validate Data ===")
	validData := map[string]interface{}{
		"model":      "gpt-4",
		"timestamp":  1234567890,
		"latency_ms": 125.5,
	}

	validateResult, err := client.ValidateData(ctx, schemaID, validData)
	if err != nil {
		log.Fatalf("Failed to validate data: %v", err)
	}
	fmt.Printf("✓ Data validation result: %v\n", validateResult.IsValid)
	if !validateResult.IsValid {
		fmt.Printf("  Errors: %v\n", validateResult.Errors)
	}
	fmt.Println()

	// Example 5: Check compatibility
	fmt.Println("=== Example 5: Check Compatibility ===")
	newSchemaContent := `{
		"type": "object",
		"properties": {
			"model": {"type": "string"},
			"timestamp": {"type": "number"},
			"latency_ms": {"type": "number"},
			"tokens": {"type": "number"}
		},
		"required": ["model", "timestamp", "latency_ms"]
	}`

	compatResult, err := client.CheckCompatibility(
		ctx,
		schemaID,
		newSchemaContent,
		schema_registry.CompatibilityBackward,
	)
	if err != nil {
		log.Fatalf("Failed to check compatibility: %v", err)
	}
	fmt.Printf("✓ Compatibility check result: %v\n", compatResult.IsCompatible)
	if !compatResult.IsCompatible {
		fmt.Printf("  Incompatibilities: %v\n", compatResult.Incompatibilities)
	}
	fmt.Println()

	// Example 6: Search schemas
	fmt.Println("=== Example 6: Search Schemas ===")
	searchResults, err := client.SearchSchemas(ctx, "telemetry", 10, 0)
	if err != nil {
		log.Fatalf("Failed to search schemas: %v", err)
	}
	fmt.Printf("✓ Found %d schemas\n", len(searchResults))
	for _, res := range searchResults {
		fmt.Printf("  - %s.%s v%s (score: %.2f)\n",
			res.Namespace, res.Name, res.Version, res.Score)
	}
	fmt.Println()

	// Example 7: List versions
	fmt.Println("=== Example 7: List Versions ===")
	versions, err := client.ListVersions(ctx, "telemetry", "InferenceEvent")
	if err != nil {
		log.Fatalf("Failed to list versions: %v", err)
	}
	fmt.Printf("✓ Found %d versions\n", len(versions))
	for _, v := range versions {
		fmt.Printf("  - Version: %s (ID: %s, Created: %s)\n",
			v.Version, v.SchemaID, v.CreatedAt)
	}
	fmt.Println()

	// Example 8: Health check
	fmt.Println("=== Example 8: Health Check ===")
	health, err := client.HealthCheck(ctx)
	if err != nil {
		log.Fatalf("Health check failed: %v", err)
	}
	fmt.Printf("✓ Service Status: %s\n", health.Status)
	if health.Version != "" {
		fmt.Printf("  Version: %s\n", health.Version)
	}
	fmt.Println()

	// Example 9: Cache statistics
	fmt.Println("=== Example 9: Cache Statistics ===")
	stats := client.CacheStats()
	fmt.Printf("✓ Cache Statistics:\n")
	fmt.Printf("  Hits: %d\n", stats.Hits)
	fmt.Printf("  Misses: %d\n", stats.Misses)
	fmt.Printf("  Evictions: %d\n", stats.Evictions)
	fmt.Println()

	fmt.Println("=== All examples completed successfully! ===")
}

func strPtr(s string) *string {
	return &s
}
