package com.llm.schema.registry.exceptions;

/**
 * Exception thrown when rate limit is exceeded.
 *
 * @since 1.0.0
 */
public class RateLimitException extends SchemaRegistryException {
    private final Integer retryAfter;

    /**
     * Constructs a new rate limit exception.
     *
     * @param retryAfter the number of seconds to wait before retrying
     */
    public RateLimitException(Integer retryAfter) {
        super("Rate limit exceeded" + (retryAfter != null ? ". Retry after " + retryAfter + " seconds" : ""), 429);
        this.retryAfter = retryAfter;
    }

    /**
     * Constructs a new rate limit exception with a custom message.
     *
     * @param message    the detail message
     * @param retryAfter the number of seconds to wait before retrying
     */
    public RateLimitException(String message, Integer retryAfter) {
        super(message, 429);
        this.retryAfter = retryAfter;
    }

    /**
     * Gets the number of seconds to wait before retrying.
     *
     * @return the retry-after value in seconds, or null if not available
     */
    public Integer getRetryAfter() {
        return retryAfter;
    }
}
