package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Supported schema formats for the LLM Schema Registry.
 *
 * @since 1.0.0
 */
public enum SchemaFormat {
    /**
     * JSON Schema format.
     */
    JSON_SCHEMA("json_schema"),

    /**
     * Apache Avro format.
     */
    AVRO("avro"),

    /**
     * Protocol Buffers format.
     */
    PROTOBUF("protobuf");

    private final String value;

    SchemaFormat(String value) {
        this.value = value;
    }

    /**
     * Gets the string value of this schema format.
     *
     * @return the string value
     */
    @JsonValue
    public String getValue() {
        return value;
    }

    /**
     * Parses a string value into a SchemaFormat enum.
     *
     * @param value the string value
     * @return the corresponding SchemaFormat
     * @throws IllegalArgumentException if the value is not recognized
     */
    public static SchemaFormat fromValue(String value) {
        for (SchemaFormat format : values()) {
            if (format.value.equals(value)) {
                return format;
            }
        }
        throw new IllegalArgumentException("Unknown schema format: " + value);
    }

    @Override
    public String toString() {
        return value;
    }
}
