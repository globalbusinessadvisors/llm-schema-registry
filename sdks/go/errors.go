package schema_registry

import (
	"errors"
	"fmt"
	"net/http"
)

// Error types for comprehensive error handling.
var (
	// ErrInvalidSchema indicates a schema validation error.
	ErrInvalidSchema = errors.New("invalid schema")
	// ErrSchemaNotFound indicates the requested schema was not found.
	ErrSchemaNotFound = errors.New("schema not found")
	// ErrIncompatibleSchema indicates schema compatibility issues.
	ErrIncompatibleSchema = errors.New("incompatible schema")
	// ErrAuthentication indicates authentication failure.
	ErrAuthentication = errors.New("authentication failed")
	// ErrAuthorization indicates insufficient permissions.
	ErrAuthorization = errors.New("insufficient permissions")
	// ErrRateLimit indicates rate limit exceeded.
	ErrRateLimit = errors.New("rate limit exceeded")
	// ErrServerError indicates a server-side error.
	ErrServerError = errors.New("server error")
	// ErrTimeout indicates a request timeout.
	ErrTimeout = errors.New("request timeout")
	// ErrCanceled indicates the request was canceled.
	ErrCanceled = errors.New("request canceled")
)

// SchemaRegistryError represents a schema registry error with additional context.
type SchemaRegistryError struct {
	// Err is the underlying error.
	Err error
	// Message provides additional context.
	Message string
	// StatusCode is the HTTP status code (if applicable).
	StatusCode int
	// RetryAfter indicates when to retry (for rate limit errors).
	RetryAfter *int
	// Details contains additional error details.
	Details map[string]interface{}
}

// Error implements the error interface.
func (e *SchemaRegistryError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("%v: %s", e.Err, e.Message)
	}
	return e.Err.Error()
}

// Unwrap returns the underlying error.
func (e *SchemaRegistryError) Unwrap() error {
	return e.Err
}

// Is implements error matching for errors.Is.
func (e *SchemaRegistryError) Is(target error) bool {
	return errors.Is(e.Err, target)
}

// NewSchemaRegistryError creates a new schema registry error.
func NewSchemaRegistryError(err error, message string, statusCode int) *SchemaRegistryError {
	return &SchemaRegistryError{
		Err:        err,
		Message:    message,
		StatusCode: statusCode,
	}
}

// SchemaNotFoundError represents a schema not found error.
type SchemaNotFoundError struct {
	// SchemaID is the ID of the schema that was not found.
	SchemaID string
}

// Error implements the error interface.
func (e *SchemaNotFoundError) Error() string {
	return fmt.Sprintf("schema not found: %s", e.SchemaID)
}

// Is implements error matching for errors.Is.
func (e *SchemaNotFoundError) Is(target error) bool {
	return target == ErrSchemaNotFound
}

// IncompatibleSchemaError represents a schema compatibility error.
type IncompatibleSchemaError struct {
	// Incompatibilities lists the compatibility issues.
	Incompatibilities []string
}

// Error implements the error interface.
func (e *IncompatibleSchemaError) Error() string {
	if len(e.Incompatibilities) == 0 {
		return "schema is incompatible"
	}
	msg := "schema compatibility check failed:\n"
	for _, inc := range e.Incompatibilities {
		msg += fmt.Sprintf("  - %s\n", inc)
	}
	return msg
}

// Is implements error matching for errors.Is.
func (e *IncompatibleSchemaError) Is(target error) bool {
	return target == ErrIncompatibleSchema
}

// ValidationError represents a schema validation error.
type ValidationError struct {
	// Errors contains the validation error messages.
	Errors []string
}

// Error implements the error interface.
func (e *ValidationError) Error() string {
	if len(e.Errors) == 0 {
		return "validation failed"
	}
	msg := "validation failed:\n"
	for _, err := range e.Errors {
		msg += fmt.Sprintf("  - %s\n", err)
	}
	return msg
}

// Is implements error matching for errors.Is.
func (e *ValidationError) Is(target error) bool {
	return target == ErrInvalidSchema
}

// RateLimitError represents a rate limit error.
type RateLimitError struct {
	// RetryAfter indicates when to retry (in seconds).
	RetryAfter *int
}

// Error implements the error interface.
func (e *RateLimitError) Error() string {
	if e.RetryAfter != nil {
		return fmt.Sprintf("rate limit exceeded, retry after %d seconds", *e.RetryAfter)
	}
	return "rate limit exceeded"
}

// Is implements error matching for errors.Is.
func (e *RateLimitError) Is(target error) bool {
	return target == ErrRateLimit
}

// mapHTTPError maps HTTP status codes to appropriate errors.
func mapHTTPError(statusCode int, body []byte) error {
	var errResp ErrorResponse
	// Try to unmarshal error response, but don't fail if we can't
	_ = unmarshalJSON(body, &errResp)

	message := errResp.Message
	if message == "" {
		message = string(body)
	}

	switch statusCode {
	case http.StatusUnauthorized:
		return NewSchemaRegistryError(ErrAuthentication, message, statusCode)
	case http.StatusForbidden:
		return NewSchemaRegistryError(ErrAuthorization, message, statusCode)
	case http.StatusNotFound:
		return &SchemaNotFoundError{SchemaID: message}
	case http.StatusConflict:
		// Check if this is an incompatibility error
		if errResp.Details != nil {
			if incomps, ok := errResp.Details["incompatibilities"].([]interface{}); ok {
				incompatibilities := make([]string, 0, len(incomps))
				for _, inc := range incomps {
					if s, ok := inc.(string); ok {
						incompatibilities = append(incompatibilities, s)
					}
				}
				return &IncompatibleSchemaError{Incompatibilities: incompatibilities}
			}
		}
		return NewSchemaRegistryError(ErrIncompatibleSchema, message, statusCode)
	case http.StatusTooManyRequests:
		// Try to extract retry-after header
		return &RateLimitError{}
	case http.StatusBadRequest:
		// Check if this is a validation error
		if errResp.Details != nil {
			if errs, ok := errResp.Details["errors"].([]interface{}); ok {
				errors := make([]string, 0, len(errs))
				for _, e := range errs {
					if s, ok := e.(string); ok {
						errors = append(errors, s)
					}
				}
				return &ValidationError{Errors: errors}
			}
		}
		return NewSchemaRegistryError(ErrInvalidSchema, message, statusCode)
	case http.StatusInternalServerError, http.StatusBadGateway, http.StatusServiceUnavailable, http.StatusGatewayTimeout:
		return NewSchemaRegistryError(ErrServerError, message, statusCode)
	default:
		return NewSchemaRegistryError(
			fmt.Errorf("unexpected status code: %d", statusCode),
			message,
			statusCode,
		)
	}
}

// IsRetryable determines if an error is retryable.
func IsRetryable(err error) bool {
	if err == nil {
		return false
	}

	// Check for specific retryable errors
	if errors.Is(err, ErrServerError) {
		return true
	}
	if errors.Is(err, ErrTimeout) {
		return true
	}

	// Check for schema registry errors with retryable status codes
	var sre *SchemaRegistryError
	if errors.As(err, &sre) {
		return sre.StatusCode >= 500 && sre.StatusCode < 600
	}

	return false
}

// IsTemporary determines if an error is temporary.
func IsTemporary(err error) bool {
	if err == nil {
		return false
	}

	// Check if error implements temporary interface
	type temporary interface {
		Temporary() bool
	}

	if te, ok := err.(temporary); ok {
		return te.Temporary()
	}

	return IsRetryable(err)
}
