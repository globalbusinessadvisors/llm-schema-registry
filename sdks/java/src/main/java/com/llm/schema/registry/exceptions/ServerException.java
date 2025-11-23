package com.llm.schema.registry.exceptions;

/**
 * Exception thrown when server returns a 5xx error.
 *
 * @since 1.0.0
 */
public class ServerException extends SchemaRegistryException {
    /**
     * Constructs a new server exception.
     *
     * @param message    the detail message
     * @param statusCode the HTTP status code
     */
    public ServerException(String message, int statusCode) {
        super(message, statusCode);
    }

    /**
     * Constructs a new server exception with a cause.
     *
     * @param message    the detail message
     * @param cause      the cause
     * @param statusCode the HTTP status code
     */
    public ServerException(String message, Throwable cause, int statusCode) {
        super(message, cause, statusCode);
    }
}
