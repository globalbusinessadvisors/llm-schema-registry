package com.llm.schema.registry.exceptions;

/**
 * Exception thrown when a requested schema is not found.
 *
 * @since 1.0.0
 */
public class SchemaNotFoundException extends SchemaRegistryException {
    private final String schemaId;

    /**
     * Constructs a new exception for a schema that was not found.
     *
     * @param schemaId the schema ID that was not found
     */
    public SchemaNotFoundException(String schemaId) {
        super("Schema not found: " + schemaId, 404);
        this.schemaId = schemaId;
    }

    /**
     * Constructs a new exception with a custom message.
     *
     * @param message  the detail message
     * @param schemaId the schema ID that was not found
     */
    public SchemaNotFoundException(String message, String schemaId) {
        super(message, 404);
        this.schemaId = schemaId;
    }

    /**
     * Gets the schema ID that was not found.
     *
     * @return the schema ID
     */
    public String getSchemaId() {
        return schemaId;
    }
}
