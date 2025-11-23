package com.llm.schema.registry.exceptions;

/**
 * Base exception for all Schema Registry errors.
 *
 * <p>All custom exceptions in the SDK extend this base class,
 * allowing clients to catch all registry-related errors with a single catch block.
 *
 * @since 1.0.0
 */
public class SchemaRegistryException extends Exception {
    private final Integer statusCode;

    /**
     * Constructs a new exception with the specified detail message.
     *
     * @param message the detail message
     */
    public SchemaRegistryException(String message) {
        this(message, null, null);
    }

    /**
     * Constructs a new exception with the specified detail message and cause.
     *
     * @param message the detail message
     * @param cause   the cause
     */
    public SchemaRegistryException(String message, Throwable cause) {
        this(message, cause, null);
    }

    /**
     * Constructs a new exception with the specified detail message and status code.
     *
     * @param message    the detail message
     * @param statusCode the HTTP status code
     */
    public SchemaRegistryException(String message, Integer statusCode) {
        this(message, null, statusCode);
    }

    /**
     * Constructs a new exception with the specified detail message, cause, and status code.
     *
     * @param message    the detail message
     * @param cause      the cause
     * @param statusCode the HTTP status code
     */
    public SchemaRegistryException(String message, Throwable cause, Integer statusCode) {
        super(message, cause);
        this.statusCode = statusCode;
    }

    /**
     * Gets the HTTP status code associated with this exception.
     *
     * @return the status code, or null if not available
     */
    public Integer getStatusCode() {
        return statusCode;
    }
}
