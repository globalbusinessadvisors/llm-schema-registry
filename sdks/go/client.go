package schema_registry

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Client is a production-ready, thread-safe client for the LLM Schema Registry.
// It supports context-based cancellation, automatic retries, and in-memory caching.
type Client struct {
	// baseURL is the base URL of the schema registry API.
	baseURL string
	// apiKey is the optional API key for authentication.
	apiKey string
	// httpClient is the underlying HTTP client.
	httpClient *http.Client
	// cache is the thread-safe LRU cache for schema responses.
	cache *Cache[string, *GetSchemaResponse]
	// retryConfig configures retry behavior.
	retryConfig RetryConfig
	// mu protects concurrent access to the client.
	mu sync.RWMutex
	// closed indicates if the client has been closed.
	closed bool
}

// ClientConfig configures the schema registry client.
type ClientConfig struct {
	// BaseURL is the base URL of the schema registry (required).
	BaseURL string
	// APIKey is the optional API key for authentication.
	APIKey string
	// HTTPClient is an optional custom HTTP client. If nil, a default client is used.
	HTTPClient *http.Client
	// Timeout is the request timeout (default: 30 seconds).
	Timeout time.Duration
	// CacheTTL is the cache time-to-live (default: 5 minutes).
	CacheTTL time.Duration
	// CacheMaxSize is the maximum number of cached items (default: 1000).
	CacheMaxSize int
	// RetryConfig configures retry behavior. If nil, default config is used.
	RetryConfig *RetryConfig
	// EnableCache enables caching (default: true).
	EnableCache bool
}

// NewClient creates a new schema registry client with the given configuration.
func NewClient(cfg ClientConfig) (*Client, error) {
	if cfg.BaseURL == "" {
		return nil, fmt.Errorf("base URL is required")
	}

	// Validate and normalize base URL
	baseURL := strings.TrimSuffix(cfg.BaseURL, "/")
	if _, err := url.Parse(baseURL); err != nil {
		return nil, fmt.Errorf("invalid base URL: %w", err)
	}

	// Set defaults
	if cfg.Timeout == 0 {
		cfg.Timeout = 30 * time.Second
	}
	if cfg.CacheTTL == 0 {
		cfg.CacheTTL = 5 * time.Minute
	}
	if cfg.CacheMaxSize == 0 {
		cfg.CacheMaxSize = 1000
	}

	// Create HTTP client if not provided
	httpClient := cfg.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{
			Timeout: cfg.Timeout,
			Transport: &http.Transport{
				MaxIdleConns:        100,
				MaxIdleConnsPerHost: 10,
				IdleConnTimeout:     90 * time.Second,
			},
		}
	}

	// Create retry config if not provided
	retryConfig := DefaultRetryConfig()
	if cfg.RetryConfig != nil {
		retryConfig = *cfg.RetryConfig
	}

	// Create cache
	var cache *Cache[string, *GetSchemaResponse]
	if cfg.EnableCache {
		cache = NewCache[string, *GetSchemaResponse](cfg.CacheMaxSize, cfg.CacheTTL, nil)
	}

	return &Client{
		baseURL:     baseURL,
		apiKey:      cfg.APIKey,
		httpClient:  httpClient,
		cache:       cache,
		retryConfig: retryConfig,
	}, nil
}

// Close closes the client and releases resources.
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return nil
	}

	c.closed = true

	// Clear cache
	if c.cache != nil {
		c.cache.Clear()
	}

	// Close idle connections
	c.httpClient.CloseIdleConnections()

	return nil
}

// RegisterSchema registers a new schema in the registry.
func (c *Client) RegisterSchema(ctx context.Context, schema *Schema) (*RegisterSchemaResponse, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	// Validate schema
	if err := schema.Validate(); err != nil {
		return nil, &ValidationError{Errors: []string{err.Error()}}
	}

	// Marshal schema
	body, err := marshalJSON(schema)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal schema: %w", err)
	}

	// Make request with retry
	resp := &RegisterSchemaResponse{}
	err = Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "POST", "/api/v1/schemas", body, resp)
	})

	if err != nil {
		return nil, err
	}

	// Invalidate cache for this schema
	if c.cache != nil {
		cacheKey := fmt.Sprintf("%s.%s", schema.Namespace, schema.Name)
		c.cache.Delete(cacheKey)
	}

	return resp, nil
}

// GetSchema retrieves a schema by ID.
func (c *Client) GetSchema(ctx context.Context, schemaID string) (*GetSchemaResponse, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	// Check cache first
	cacheKey := fmt.Sprintf("schema:%s", schemaID)
	if c.cache != nil {
		if cached, ok := c.cache.Get(cacheKey); ok {
			return cached, nil
		}
	}

	// Make request with retry
	resp := &GetSchemaResponse{}
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "GET", fmt.Sprintf("/api/v1/schemas/%s", schemaID), nil, resp)
	})

	if err != nil {
		return nil, err
	}

	// Cache the result
	if c.cache != nil {
		c.cache.Set(cacheKey, resp)
	}

	return resp, nil
}

// GetSchemaByVersion retrieves a schema by namespace, name, and version.
func (c *Client) GetSchemaByVersion(ctx context.Context, namespace, name, version string) (*GetSchemaResponse, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	// Check cache first
	cacheKey := fmt.Sprintf("schema:%s.%s:%s", namespace, name, version)
	if c.cache != nil {
		if cached, ok := c.cache.Get(cacheKey); ok {
			return cached, nil
		}
	}

	// Make request with retry
	path := fmt.Sprintf("/api/v1/schemas/%s/%s/versions/%s", namespace, name, version)
	resp := &GetSchemaResponse{}
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "GET", path, nil, resp)
	})

	if err != nil {
		return nil, err
	}

	// Cache the result
	if c.cache != nil {
		c.cache.Set(cacheKey, resp)
	}

	return resp, nil
}

// ValidateData validates data against a schema.
func (c *Client) ValidateData(ctx context.Context, schemaID string, data interface{}) (*ValidateResponse, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	// Prepare payload
	var dataStr string
	switch v := data.(type) {
	case string:
		dataStr = v
	default:
		dataBytes, err := marshalJSON(data)
		if err != nil {
			return nil, fmt.Errorf("failed to marshal data: %w", err)
		}
		dataStr = string(dataBytes)
	}

	payload := map[string]interface{}{
		"schema_id": schemaID,
		"data":      dataStr,
	}

	body, err := marshalJSON(payload)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal payload: %w", err)
	}

	// Make request with retry
	resp := &ValidateResponse{}
	err = Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "POST", "/api/v1/validate", body, resp)
	})

	return resp, err
}

// CheckCompatibility checks if a new schema is compatible with an existing schema.
func (c *Client) CheckCompatibility(ctx context.Context, schemaID, newSchemaContent string, mode CompatibilityMode) (*CompatibilityResult, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	payload := map[string]interface{}{
		"schema_id":  schemaID,
		"new_schema": newSchemaContent,
		"mode":       mode.String(),
	}

	body, err := marshalJSON(payload)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal payload: %w", err)
	}

	// Make request with retry
	resp := &CompatibilityResult{}
	err = Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "POST", "/api/v1/compatibility/check", body, resp)
	})

	return resp, err
}

// SearchSchemas searches for schemas using full-text search.
func (c *Client) SearchSchemas(ctx context.Context, query string, limit, offset int) ([]*SearchResult, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	// Build query parameters
	params := url.Values{}
	params.Set("q", query)
	params.Set("limit", strconv.Itoa(limit))
	params.Set("offset", strconv.Itoa(offset))

	path := fmt.Sprintf("/api/v1/search?%s", params.Encode())

	// Make request with retry
	var resp []*SearchResult
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "GET", path, nil, &resp)
	})

	return resp, err
}

// ListVersions lists all versions of a schema.
func (c *Client) ListVersions(ctx context.Context, namespace, name string) ([]*SchemaVersion, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	path := fmt.Sprintf("/api/v1/schemas/%s/%s/versions", namespace, name)

	// Make request with retry
	var resp []*SchemaVersion
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "GET", path, nil, &resp)
	})

	return resp, err
}

// DeleteSchema deletes a schema by ID.
func (c *Client) DeleteSchema(ctx context.Context, schemaID string) error {
	if err := c.checkClosed(); err != nil {
		return err
	}

	// Make request with retry
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "DELETE", fmt.Sprintf("/api/v1/schemas/%s", schemaID), nil, nil)
	})

	if err != nil {
		return err
	}

	// Invalidate cache
	if c.cache != nil {
		cacheKey := fmt.Sprintf("schema:%s", schemaID)
		c.cache.Delete(cacheKey)
	}

	return nil
}

// HealthCheck checks the health of the schema registry.
func (c *Client) HealthCheck(ctx context.Context) (*HealthResponse, error) {
	if err := c.checkClosed(); err != nil {
		return nil, err
	}

	resp := &HealthResponse{}
	err := Retry(ctx, c.retryConfig, func() error {
		return c.doRequest(ctx, "GET", "/health", nil, resp)
	})

	return resp, err
}

// ClearCache clears the in-memory cache.
func (c *Client) ClearCache() {
	if c.cache != nil {
		c.cache.Clear()
	}
}

// CacheStats returns cache statistics.
func (c *Client) CacheStats() CacheStats {
	if c.cache != nil {
		return c.cache.Stats()
	}
	return CacheStats{}
}

// doRequest performs an HTTP request and unmarshals the response.
// result must be a pointer to the target struct or nil for no response body.
func (c *Client) doRequest(ctx context.Context, method, path string, body []byte, result interface{}) error {
	// Build full URL
	fullURL := c.baseURL + path

	// Create request
	var bodyReader io.Reader
	if body != nil {
		bodyReader = bytes.NewReader(body)
	}

	req, err := http.NewRequestWithContext(ctx, method, fullURL, bodyReader)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	// Set headers
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")
	if c.apiKey != "" {
		req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", c.apiKey))
	}

	// Execute request
	resp, err := c.httpClient.Do(req)
	if err != nil {
		// Check if context was canceled
		if ctx.Err() != nil {
			return ctx.Err()
		}
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	// Read response body
	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response body: %w", err)
	}

	// Check for errors
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return mapHTTPError(resp.StatusCode, respBody)
	}

	// Unmarshal response (if result is not nil)
	if result != nil {
		if err := unmarshalJSON(respBody, result); err != nil {
			return fmt.Errorf("failed to unmarshal response: %w", err)
		}
	}

	return nil
}

// checkClosed checks if the client has been closed.
func (c *Client) checkClosed() error {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if c.closed {
		return fmt.Errorf("client is closed")
	}
	return nil
}

// marshalJSON marshals a value to JSON.
func marshalJSON(v interface{}) ([]byte, error) {
	return json.Marshal(v)
}

// unmarshalJSON unmarshals JSON to a value.
func unmarshalJSON(data []byte, v interface{}) error {
	return json.Unmarshal(data, v)
}
