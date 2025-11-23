package com.llm.schema.registry.exceptions;

/**
 * Exception thrown when authorization fails (insufficient permissions).
 *
 * @since 1.0.0
 */
public class AuthorizationException extends SchemaRegistryException {
    /**
     * Constructs a new authorization exception.
     *
     * @param message the detail message
     */
    public AuthorizationException(String message) {
        super(message, 403);
    }

    /**
     * Constructs a new authorization exception with a cause.
     *
     * @param message the detail message
     * @param cause   the cause
     */
    public AuthorizationException(String message, Throwable cause) {
        super(message, cause, 403);
    }
}
