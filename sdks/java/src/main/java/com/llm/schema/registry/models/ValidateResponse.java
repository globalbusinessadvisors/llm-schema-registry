package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * Response from data validation operation.
 *
 * @since 1.0.0
 */
public final class ValidateResponse {
    @JsonProperty("is_valid")
    private final boolean valid;

    @JsonProperty("errors")
    private final List<String> errors;

    /**
     * Creates a new ValidateResponse.
     *
     * @param valid  whether the data is valid
     * @param errors list of validation error messages
     */
    @JsonCreator
    public ValidateResponse(
            @JsonProperty("is_valid") boolean valid,
            @JsonProperty("errors") List<String> errors) {
        this.valid = valid;
        this.errors = errors != null ? Collections.unmodifiableList(new ArrayList<>(errors)) : Collections.emptyList();
    }

    /**
     * Checks if the data is valid.
     *
     * @return true if valid, false otherwise
     */
    public boolean isValid() {
        return valid;
    }

    /**
     * Gets validation error messages.
     *
     * @return an unmodifiable list of error messages
     */
    @NotNull
    public List<String> getErrors() {
        return errors;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        ValidateResponse that = (ValidateResponse) o;
        return valid == that.valid && Objects.equals(errors, that.errors);
    }

    @Override
    public int hashCode() {
        return Objects.hash(valid, errors);
    }

    @Override
    public String toString() {
        return "ValidateResponse{" +
                "valid=" + valid +
                ", errors=" + errors +
                '}';
    }
}
