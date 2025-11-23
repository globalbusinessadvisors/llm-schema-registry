package com.llm.schema.registry.exceptions;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Exception thrown when schema compatibility check fails.
 *
 * @since 1.0.0
 */
public class IncompatibleSchemaException extends SchemaRegistryException {
    private final List<String> incompatibilities;

    /**
     * Constructs a new exception with incompatibility issues.
     *
     * @param incompatibilities the list of incompatibility issues
     */
    public IncompatibleSchemaException(List<String> incompatibilities) {
        super("Schema is incompatible: " + String.join(", ", incompatibilities), 409);
        this.incompatibilities = incompatibilities != null ?
                Collections.unmodifiableList(new ArrayList<>(incompatibilities)) :
                Collections.emptyList();
    }

    /**
     * Constructs a new exception with a custom message and incompatibility issues.
     *
     * @param message           the detail message
     * @param incompatibilities the list of incompatibility issues
     */
    public IncompatibleSchemaException(String message, List<String> incompatibilities) {
        super(message, 409);
        this.incompatibilities = incompatibilities != null ?
                Collections.unmodifiableList(new ArrayList<>(incompatibilities)) :
                Collections.emptyList();
    }

    /**
     * Gets the list of incompatibility issues.
     *
     * @return an unmodifiable list of incompatibilities
     */
    public List<String> getIncompatibilities() {
        return incompatibilities;
    }
}
