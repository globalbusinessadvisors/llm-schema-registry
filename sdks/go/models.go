package schema_registry

import (
	"encoding/json"
	"fmt"
	"regexp"
	"time"
)

// SchemaFormat represents the supported schema formats.
type SchemaFormat string

const (
	// SchemaFormatJSONSchema represents JSON Schema format.
	SchemaFormatJSONSchema SchemaFormat = "json_schema"
	// SchemaFormatAvro represents Apache Avro format.
	SchemaFormatAvro SchemaFormat = "avro"
	// SchemaFormatProtobuf represents Protocol Buffers format.
	SchemaFormatProtobuf SchemaFormat = "protobuf"
)

// String returns the string representation of the schema format.
func (f SchemaFormat) String() string {
	return string(f)
}

// CompatibilityMode represents schema compatibility checking modes.
type CompatibilityMode string

const (
	// CompatibilityBackward allows deletion of fields.
	CompatibilityBackward CompatibilityMode = "backward"
	// CompatibilityForward allows addition of fields.
	CompatibilityForward CompatibilityMode = "forward"
	// CompatibilityFull requires both backward and forward compatibility.
	CompatibilityFull CompatibilityMode = "full"
	// CompatibilityBackwardTransitive checks backward compatibility transitively.
	CompatibilityBackwardTransitive CompatibilityMode = "backward_transitive"
	// CompatibilityForwardTransitive checks forward compatibility transitively.
	CompatibilityForwardTransitive CompatibilityMode = "forward_transitive"
	// CompatibilityFullTransitive checks full compatibility transitively.
	CompatibilityFullTransitive CompatibilityMode = "full_transitive"
	// CompatibilityNone disables compatibility checking.
	CompatibilityNone CompatibilityMode = "none"
)

// String returns the string representation of the compatibility mode.
func (m CompatibilityMode) String() string {
	return string(m)
}

// SchemaMetadata contains metadata for a schema.
type SchemaMetadata struct {
	// Description is a human-readable description of the schema.
	Description *string `json:"description,omitempty"`
	// Tags are labels for categorization and search.
	Tags []string `json:"tags,omitempty"`
	// Owner is the owner/team responsible for the schema.
	Owner *string `json:"owner,omitempty"`
	// Custom contains additional custom metadata.
	Custom map[string]interface{} `json:"custom,omitempty"`
}

// Schema represents a schema definition.
type Schema struct {
	// Namespace is the schema namespace (e.g., "telemetry").
	Namespace string `json:"namespace"`
	// Name is the schema name (e.g., "InferenceEvent").
	Name string `json:"name"`
	// Version is the semantic version (e.g., "1.0.0").
	Version string `json:"version"`
	// Format is the schema format.
	Format SchemaFormat `json:"format"`
	// Content is the schema content (JSON/Avro/Protobuf).
	Content string `json:"content"`
	// Metadata contains optional metadata.
	Metadata *SchemaMetadata `json:"metadata,omitempty"`
}

var semverRegex = regexp.MustCompile(`^\d+\.\d+\.\d+$`)

// Validate validates the schema fields.
func (s *Schema) Validate() error {
	if s.Namespace == "" {
		return fmt.Errorf("namespace is required")
	}
	if s.Name == "" {
		return fmt.Errorf("name is required")
	}
	if s.Version == "" {
		return fmt.Errorf("version is required")
	}
	if !semverRegex.MatchString(s.Version) {
		return fmt.Errorf("version must be in semver format (major.minor.patch): %s", s.Version)
	}
	if s.Format == "" {
		return fmt.Errorf("format is required")
	}
	if s.Content == "" {
		return fmt.Errorf("content is required")
	}
	return nil
}

// RegisterSchemaResponse represents the response from schema registration.
type RegisterSchemaResponse struct {
	// SchemaID is the unique schema ID (UUID).
	SchemaID string `json:"schema_id"`
	// Version is the registered version.
	Version string `json:"version"`
	// CreatedAt is the creation timestamp.
	CreatedAt time.Time `json:"created_at"`
}

// GetSchemaResponse represents the response from schema retrieval.
type GetSchemaResponse struct {
	// SchemaID is the unique schema ID.
	SchemaID string `json:"schema_id"`
	// Namespace is the schema namespace.
	Namespace string `json:"namespace"`
	// Name is the schema name.
	Name string `json:"name"`
	// Version is the schema version.
	Version string `json:"version"`
	// Format is the schema format.
	Format SchemaFormat `json:"format"`
	// Content is the schema content.
	Content string `json:"content"`
	// Metadata contains optional metadata.
	Metadata *SchemaMetadata `json:"metadata,omitempty"`
	// CreatedAt is the creation timestamp.
	CreatedAt time.Time `json:"created_at"`
	// UpdatedAt is the last update timestamp.
	UpdatedAt time.Time `json:"updated_at"`
}

// ValidateResponse represents the response from data validation.
type ValidateResponse struct {
	// IsValid indicates whether the data is valid.
	IsValid bool `json:"is_valid"`
	// Errors contains validation error messages.
	Errors []string `json:"errors,omitempty"`
}

// CompatibilityResult represents the result of compatibility checking.
type CompatibilityResult struct {
	// IsCompatible indicates whether the schemas are compatible.
	IsCompatible bool `json:"is_compatible"`
	// Incompatibilities lists incompatibility issues.
	Incompatibilities []string `json:"incompatibilities,omitempty"`
	// Mode is the compatibility mode used.
	Mode CompatibilityMode `json:"mode"`
}

// SchemaVersion represents schema version information.
type SchemaVersion struct {
	// Version is the schema version.
	Version string `json:"version"`
	// SchemaID is the unique schema ID.
	SchemaID string `json:"schema_id"`
	// CreatedAt is the creation timestamp.
	CreatedAt time.Time `json:"created_at"`
}

// SearchResult represents a schema search result.
type SearchResult struct {
	// SchemaID is the unique schema ID.
	SchemaID string `json:"schema_id"`
	// Namespace is the schema namespace.
	Namespace string `json:"namespace"`
	// Name is the schema name.
	Name string `json:"name"`
	// Version is the schema version.
	Version string `json:"version"`
	// Description is the schema description.
	Description *string `json:"description,omitempty"`
	// Tags are the schema tags.
	Tags []string `json:"tags,omitempty"`
	// Score is the relevance score.
	Score float64 `json:"score"`
}

// HealthResponse represents the health check response.
type HealthResponse struct {
	// Status is the service status.
	Status string `json:"status"`
	// Version is the service version.
	Version string `json:"version,omitempty"`
	// Uptime is the service uptime in seconds.
	Uptime int64 `json:"uptime,omitempty"`
	// Details contains additional health details.
	Details map[string]interface{} `json:"details,omitempty"`
}

// ErrorResponse represents an error response from the API.
type ErrorResponse struct {
	// Message is the error message.
	Message string `json:"message"`
	// Code is the error code.
	Code string `json:"code,omitempty"`
	// Details contains additional error details.
	Details map[string]interface{} `json:"details,omitempty"`
}

// Error implements the error interface for ErrorResponse.
func (e *ErrorResponse) Error() string {
	if e.Code != "" {
		return fmt.Sprintf("%s: %s", e.Code, e.Message)
	}
	return e.Message
}

// UnmarshalJSON implements custom JSON unmarshaling for ErrorResponse.
func (e *ErrorResponse) UnmarshalJSON(data []byte) error {
	type Alias ErrorResponse
	aux := &struct {
		*Alias
	}{
		Alias: (*Alias)(e),
	}
	if err := json.Unmarshal(data, &aux); err != nil {
		return err
	}
	return nil
}
