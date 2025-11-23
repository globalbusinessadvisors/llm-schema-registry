package schema_registry

import (
	"context"
	"math"
	"math/rand"
	"time"
)

// RetryConfig configures retry behavior.
type RetryConfig struct {
	// MaxAttempts is the maximum number of retry attempts (0 means no retries).
	MaxAttempts int
	// InitialBackoff is the initial backoff duration.
	InitialBackoff time.Duration
	// MaxBackoff is the maximum backoff duration.
	MaxBackoff time.Duration
	// BackoffMultiplier is the multiplier for exponential backoff.
	BackoffMultiplier float64
	// Jitter adds randomness to backoff to prevent thundering herd.
	Jitter bool
	// RetryableFunc determines if an error is retryable.
	RetryableFunc func(error) bool
}

// DefaultRetryConfig returns the default retry configuration.
func DefaultRetryConfig() RetryConfig {
	return RetryConfig{
		MaxAttempts:       3,
		InitialBackoff:    500 * time.Millisecond,
		MaxBackoff:        10 * time.Second,
		BackoffMultiplier: 2.0,
		Jitter:            true,
		RetryableFunc:     IsRetryable,
	}
}

// Retry executes a function with exponential backoff retry logic.
// It respects context cancellation and returns early if the context is canceled.
func Retry(ctx context.Context, cfg RetryConfig, fn func() error) error {
	if cfg.MaxAttempts <= 0 {
		// No retries, execute once
		return fn()
	}

	var lastErr error
	backoff := cfg.InitialBackoff

	for attempt := 0; attempt <= cfg.MaxAttempts; attempt++ {
		// Execute the function
		err := fn()
		if err == nil {
			return nil
		}

		lastErr = err

		// Check if we should retry
		if attempt < cfg.MaxAttempts && cfg.RetryableFunc(err) {
			// Calculate backoff duration
			waitDuration := calculateBackoff(backoff, cfg.MaxBackoff, cfg.Jitter)

			// Wait with context cancellation support
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(waitDuration):
				// Continue to next attempt
			}

			// Increase backoff for next iteration
			backoff = time.Duration(float64(backoff) * cfg.BackoffMultiplier)
		} else {
			// Don't retry
			break
		}
	}

	return lastErr
}

// RetryWithResult executes a function with exponential backoff retry logic and returns a result.
// This uses generics to support any return type.
func RetryWithResult[T any](ctx context.Context, cfg RetryConfig, fn func() (T, error)) (T, error) {
	var result T
	var lastErr error

	if cfg.MaxAttempts <= 0 {
		// No retries, execute once
		return fn()
	}

	backoff := cfg.InitialBackoff

	for attempt := 0; attempt <= cfg.MaxAttempts; attempt++ {
		// Execute the function
		res, err := fn()
		if err == nil {
			return res, nil
		}

		result = res
		lastErr = err

		// Check if we should retry
		if attempt < cfg.MaxAttempts && cfg.RetryableFunc(err) {
			// Calculate backoff duration
			waitDuration := calculateBackoff(backoff, cfg.MaxBackoff, cfg.Jitter)

			// Wait with context cancellation support
			select {
			case <-ctx.Done():
				return result, ctx.Err()
			case <-time.After(waitDuration):
				// Continue to next attempt
			}

			// Increase backoff for next iteration
			backoff = time.Duration(float64(backoff) * cfg.BackoffMultiplier)
		} else {
			// Don't retry
			break
		}
	}

	return result, lastErr
}

// calculateBackoff calculates the backoff duration with optional jitter.
func calculateBackoff(backoff, maxBackoff time.Duration, jitter bool) time.Duration {
	// Apply max backoff limit
	if backoff > maxBackoff {
		backoff = maxBackoff
	}

	// Add jitter if enabled
	if jitter {
		// Add random jitter between 0 and 25% of the backoff
		jitterAmount := time.Duration(rand.Float64() * float64(backoff) * 0.25)
		backoff += jitterAmount
	}

	return backoff
}

// ExponentialBackoff calculates exponential backoff duration.
// attempt: the current attempt number (0-indexed)
// initialBackoff: the initial backoff duration
// maxBackoff: the maximum backoff duration
func ExponentialBackoff(attempt int, initialBackoff, maxBackoff time.Duration) time.Duration {
	backoff := time.Duration(float64(initialBackoff) * math.Pow(2, float64(attempt)))
	if backoff > maxBackoff {
		backoff = maxBackoff
	}
	return backoff
}

// ExponentialBackoffWithJitter calculates exponential backoff duration with jitter.
func ExponentialBackoffWithJitter(attempt int, initialBackoff, maxBackoff time.Duration) time.Duration {
	backoff := ExponentialBackoff(attempt, initialBackoff, maxBackoff)
	jitter := time.Duration(rand.Float64() * float64(backoff) * 0.25)
	return backoff + jitter
}

// Backoff provides a simple interface for exponential backoff.
type Backoff struct {
	attempt           int
	initialBackoff    time.Duration
	maxBackoff        time.Duration
	backoffMultiplier float64
	jitter            bool
}

// NewBackoff creates a new Backoff instance.
func NewBackoff(initialBackoff, maxBackoff time.Duration, jitter bool) *Backoff {
	return &Backoff{
		attempt:           0,
		initialBackoff:    initialBackoff,
		maxBackoff:        maxBackoff,
		backoffMultiplier: 2.0,
		jitter:            jitter,
	}
}

// Next returns the next backoff duration and increments the attempt counter.
func (b *Backoff) Next() time.Duration {
	backoff := time.Duration(float64(b.initialBackoff) * math.Pow(b.backoffMultiplier, float64(b.attempt)))
	if backoff > b.maxBackoff {
		backoff = b.maxBackoff
	}

	if b.jitter {
		jitter := time.Duration(rand.Float64() * float64(backoff) * 0.25)
		backoff += jitter
	}

	b.attempt++
	return backoff
}

// Reset resets the backoff to the initial state.
func (b *Backoff) Reset() {
	b.attempt = 0
}

// Attempt returns the current attempt number.
func (b *Backoff) Attempt() int {
	return b.attempt
}
