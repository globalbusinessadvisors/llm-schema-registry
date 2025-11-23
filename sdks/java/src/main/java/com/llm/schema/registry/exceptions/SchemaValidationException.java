package com.llm.schema.registry.exceptions;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Exception thrown when schema validation fails.
 *
 * @since 1.0.0
 */
public class SchemaValidationException extends SchemaRegistryException {
    private final List<String> validationErrors;

    /**
     * Constructs a new exception with validation errors.
     *
     * @param message          the detail message
     * @param validationErrors the list of validation error messages
     */
    public SchemaValidationException(String message, List<String> validationErrors) {
        super(message, 400);
        this.validationErrors = validationErrors != null ?
                Collections.unmodifiableList(new ArrayList<>(validationErrors)) :
                Collections.emptyList();
    }

    /**
     * Constructs a new exception with a single validation error.
     *
     * @param message          the detail message
     * @param validationError the validation error message
     */
    public SchemaValidationException(String message, String validationError) {
        this(message, Collections.singletonList(validationError));
    }

    /**
     * Gets the list of validation error messages.
     *
     * @return an unmodifiable list of validation errors
     */
    public List<String> getValidationErrors() {
        return validationErrors;
    }
}
