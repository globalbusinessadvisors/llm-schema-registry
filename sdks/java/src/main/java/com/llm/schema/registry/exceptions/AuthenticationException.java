package com.llm.schema.registry.exceptions;

/**
 * Exception thrown when authentication fails.
 *
 * @since 1.0.0
 */
public class AuthenticationException extends SchemaRegistryException {
    /**
     * Constructs a new authentication exception.
     *
     * @param message the detail message
     */
    public AuthenticationException(String message) {
        super(message, 401);
    }

    /**
     * Constructs a new authentication exception with a cause.
     *
     * @param message the detail message
     * @param cause   the cause
     */
    public AuthenticationException(String message, Throwable cause) {
        super(message, cause, 401);
    }
}
