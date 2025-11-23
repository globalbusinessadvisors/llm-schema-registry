package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Schema compatibility checking modes.
 *
 * <p>These modes determine how schema evolution is validated:
 * <ul>
 *     <li>{@link #BACKWARD} - New schema can read data written by old schema</li>
 *     <li>{@link #FORWARD} - Old schema can read data written by new schema</li>
 *     <li>{@link #FULL} - Both backward and forward compatible</li>
 *     <li>{@link #BACKWARD_TRANSITIVE} - Backward compatible with all previous versions</li>
 *     <li>{@link #FORWARD_TRANSITIVE} - Forward compatible with all previous versions</li>
 *     <li>{@link #FULL_TRANSITIVE} - Full compatibility with all previous versions</li>
 *     <li>{@link #NONE} - No compatibility checking</li>
 * </ul>
 *
 * @since 1.0.0
 */
public enum CompatibilityMode {
    /**
     * New schema can read data written by old schema.
     */
    BACKWARD("backward"),

    /**
     * Old schema can read data written by new schema.
     */
    FORWARD("forward"),

    /**
     * Both backward and forward compatible.
     */
    FULL("full"),

    /**
     * Backward compatible with all previous versions.
     */
    BACKWARD_TRANSITIVE("backward_transitive"),

    /**
     * Forward compatible with all previous versions.
     */
    FORWARD_TRANSITIVE("forward_transitive"),

    /**
     * Full compatibility with all previous versions.
     */
    FULL_TRANSITIVE("full_transitive"),

    /**
     * No compatibility checking.
     */
    NONE("none");

    private final String value;

    CompatibilityMode(String value) {
        this.value = value;
    }

    /**
     * Gets the string value of this compatibility mode.
     *
     * @return the string value
     */
    @JsonValue
    public String getValue() {
        return value;
    }

    /**
     * Parses a string value into a CompatibilityMode enum.
     *
     * @param value the string value
     * @return the corresponding CompatibilityMode
     * @throws IllegalArgumentException if the value is not recognized
     */
    public static CompatibilityMode fromValue(String value) {
        for (CompatibilityMode mode : values()) {
            if (mode.value.equals(value)) {
                return mode;
            }
        }
        throw new IllegalArgumentException("Unknown compatibility mode: " + value);
    }

    @Override
    public String toString() {
        return value;
    }
}
