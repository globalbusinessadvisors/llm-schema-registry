package schema_registry

import (
	"context"
	"testing"
	"time"
)

// TestClientCreation tests that a client can be created successfully
func TestClientCreation(t *testing.T) {
	client, err := NewClient(ClientConfig{
		BaseURL: "http://localhost:8080",
	})
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()
}

// TestSchemaValidation tests schema validation
func TestSchemaValidation(t *testing.T) {
	schema := &Schema{
		Namespace: "test",
		Name:      "TestSchema",
		Version:   "1.0.0",
		Format:    SchemaFormatJSONSchema,
		Content:   `{"type": "object"}`,
	}

	if err := schema.Validate(); err != nil {
		t.Errorf("Schema validation failed: %v", err)
	}
}

// TestSchemaValidationErrors tests that invalid schemas are rejected
func TestSchemaValidationErrors(t *testing.T) {
	tests := []struct {
		name   string
		schema *Schema
	}{
		{
			name: "missing namespace",
			schema: &Schema{
				Name:    "Test",
				Version: "1.0.0",
				Format:  SchemaFormatJSONSchema,
				Content: "{}",
			},
		},
		{
			name: "invalid version",
			schema: &Schema{
				Namespace: "test",
				Name:      "Test",
				Version:   "invalid",
				Format:    SchemaFormatJSONSchema,
				Content:   "{}",
			},
		},
		{
			name: "missing content",
			schema: &Schema{
				Namespace: "test",
				Name:      "Test",
				Version:   "1.0.0",
				Format:    SchemaFormatJSONSchema,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := tt.schema.Validate(); err == nil {
				t.Error("Expected validation error, got nil")
			}
		})
	}
}

// TestCacheOperations tests cache functionality
func TestCacheOperations(t *testing.T) {
	cache := NewCache[string, string](10, 5*time.Minute, nil)

	// Set and get
	cache.Set("key1", "value1")
	if val, ok := cache.Get("key1"); !ok || val != "value1" {
		t.Errorf("Expected value1, got %s", val)
	}

	// Stats
	stats := cache.Stats()
	if stats.Hits != 1 {
		t.Errorf("Expected 1 hit, got %d", stats.Hits)
	}

	// Clear
	cache.Clear()
	if cache.Len() != 0 {
		t.Error("Expected cache to be empty after clear")
	}
}

// TestCacheTTL tests cache TTL expiration
func TestCacheTTL(t *testing.T) {
	cache := NewCache[string, string](10, 100*time.Millisecond, nil)

	cache.Set("key1", "value1")
	if _, ok := cache.Get("key1"); !ok {
		t.Error("Expected to find key1")
	}

	// Wait for expiration
	time.Sleep(150 * time.Millisecond)

	if val, ok := cache.Get("key1"); ok {
		t.Errorf("Expected key1 to be expired, got %s", val)
	}
}

// TestCacheEviction tests LRU eviction
func TestCacheEviction(t *testing.T) {
	cache := NewCache[int, string](3, 5*time.Minute, nil)

	// Fill cache
	cache.Set(1, "one")
	cache.Set(2, "two")
	cache.Set(3, "three")

	// Add one more, should evict oldest (1)
	cache.Set(4, "four")

	if _, ok := cache.Get(1); ok {
		t.Error("Expected key 1 to be evicted")
	}
	if _, ok := cache.Get(4); !ok {
		t.Error("Expected key 4 to exist")
	}
}

// TestErrorTypes tests error type assertions
func TestErrorTypes(t *testing.T) {
	notFoundErr := &SchemaNotFoundError{SchemaID: "test-id"}
	if notFoundErr.Error() == "" {
		t.Error("Error message should not be empty")
	}

	validationErr := &ValidationError{Errors: []string{"error1"}}
	if validationErr.Error() == "" {
		t.Error("Error message should not be empty")
	}

	incompatibleErr := &IncompatibleSchemaError{
		Incompatibilities: []string{"incompatibility1"},
	}
	if incompatibleErr.Error() == "" {
		t.Error("Error message should not be empty")
	}

	rateLimitErr := &RateLimitError{}
	if rateLimitErr.Error() == "" {
		t.Error("Error message should not be empty")
	}
}

// TestContextCancellation tests context cancellation
func TestContextCancellation(t *testing.T) {
	client, err := NewClient(ClientConfig{
		BaseURL: "http://localhost:8080",
		Timeout: 1 * time.Second,
	})
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately

	// This should fail with context.Canceled
	_, err = client.GetSchema(ctx, "some-id")
	if err == nil {
		t.Error("Expected error due to canceled context")
	}
}

// TestRetryConfiguration tests retry configuration
func TestRetryConfiguration(t *testing.T) {
	config := DefaultRetryConfig()
	if config.MaxAttempts != 3 {
		t.Errorf("Expected 3 max attempts, got %d", config.MaxAttempts)
	}
	if config.InitialBackoff != 500*time.Millisecond {
		t.Errorf("Expected 500ms initial backoff, got %v", config.InitialBackoff)
	}
	if config.BackoffMultiplier != 2.0 {
		t.Errorf("Expected 2.0 backoff multiplier, got %v", config.BackoffMultiplier)
	}
}

// TestThreadSafety tests concurrent cache access
func TestThreadSafety(t *testing.T) {
	cache := NewCache[int, string](100, 5*time.Minute, nil)

	done := make(chan bool)
	for i := 0; i < 10; i++ {
		go func(id int) {
			for j := 0; j < 100; j++ {
				cache.Set(id*100+j, "value")
				_, _ = cache.Get(id * 100)
			}
			done <- true
		}(i)
	}

	for i := 0; i < 10; i++ {
		<-done
	}

	// Just verify no panic occurred
	stats := cache.Stats()
	if stats.Hits == 0 && stats.Misses == 0 {
		t.Error("Expected some cache activity")
	}
}

// TestIsRetryable tests retryable error detection
func TestIsRetryable(t *testing.T) {
	tests := []struct {
		name     string
		err      error
		expected bool
	}{
		{"nil error", nil, false},
		{"server error", ErrServerError, true},
		{"timeout error", ErrTimeout, true},
		{"authentication error", ErrAuthentication, false},
		{"not found error", ErrSchemaNotFound, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := IsRetryable(tt.err); got != tt.expected {
				t.Errorf("IsRetryable() = %v, want %v", got, tt.expected)
			}
		})
	}
}

// TestSchemaFormats tests schema format constants
func TestSchemaFormats(t *testing.T) {
	formats := []SchemaFormat{
		SchemaFormatJSONSchema,
		SchemaFormatAvro,
		SchemaFormatProtobuf,
	}

	for _, format := range formats {
		if format.String() == "" {
			t.Errorf("Format %v should have non-empty string representation", format)
		}
	}
}

// TestCompatibilityModes tests compatibility mode constants
func TestCompatibilityModes(t *testing.T) {
	modes := []CompatibilityMode{
		CompatibilityBackward,
		CompatibilityForward,
		CompatibilityFull,
		CompatibilityBackwardTransitive,
		CompatibilityForwardTransitive,
		CompatibilityFullTransitive,
		CompatibilityNone,
	}

	for _, mode := range modes {
		if mode.String() == "" {
			t.Errorf("Mode %v should have non-empty string representation", mode)
		}
	}
}

// TestBackoffCalculation tests exponential backoff calculation
func TestBackoffCalculation(t *testing.T) {
	tests := []struct {
		attempt        int
		initialBackoff time.Duration
		maxBackoff     time.Duration
		expectedMax    time.Duration
	}{
		{0, 100 * time.Millisecond, 10 * time.Second, 100 * time.Millisecond},
		{1, 100 * time.Millisecond, 10 * time.Second, 200 * time.Millisecond},
		{2, 100 * time.Millisecond, 10 * time.Second, 400 * time.Millisecond},
		{10, 100 * time.Millisecond, 1 * time.Second, 1 * time.Second}, // Should cap at max
	}

	for _, tt := range tests {
		t.Run("", func(t *testing.T) {
			backoff := ExponentialBackoff(tt.attempt, tt.initialBackoff, tt.maxBackoff)
			if backoff > tt.expectedMax {
				t.Errorf("Backoff %v exceeds expected max %v", backoff, tt.expectedMax)
			}
		})
	}
}

// TestClientClose tests that closing a client multiple times is safe
func TestClientClose(t *testing.T) {
	client, err := NewClient(ClientConfig{
		BaseURL: "http://localhost:8080",
	})
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}

	// Close multiple times should not panic
	if err := client.Close(); err != nil {
		t.Errorf("First close failed: %v", err)
	}
	if err := client.Close(); err != nil {
		t.Errorf("Second close failed: %v", err)
	}
}

// TestClientOperationsAfterClose tests that operations fail after close
func TestClientOperationsAfterClose(t *testing.T) {
	client, err := NewClient(ClientConfig{
		BaseURL: "http://localhost:8080",
	})
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}

	client.Close()

	// Operations should fail after close
	ctx := context.Background()
	_, err = client.GetSchema(ctx, "some-id")
	if err == nil {
		t.Error("Expected error when using closed client")
	}
}
