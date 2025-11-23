package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * Result of compatibility checking operation.
 *
 * @since 1.0.0
 */
public final class CompatibilityResult {
    @JsonProperty("is_compatible")
    private final boolean compatible;

    @JsonProperty("incompatibilities")
    private final List<String> incompatibilities;

    @JsonProperty("mode")
    private final CompatibilityMode mode;

    /**
     * Creates a new CompatibilityResult.
     *
     * @param compatible        whether the schemas are compatible
     * @param incompatibilities list of incompatibility issues
     * @param mode              the compatibility mode used
     */
    @JsonCreator
    public CompatibilityResult(
            @JsonProperty("is_compatible") boolean compatible,
            @JsonProperty("incompatibilities") List<String> incompatibilities,
            @JsonProperty("mode") CompatibilityMode mode) {
        this.compatible = compatible;
        this.incompatibilities = incompatibilities != null ?
                Collections.unmodifiableList(new ArrayList<>(incompatibilities)) :
                Collections.emptyList();
        this.mode = Objects.requireNonNull(mode, "mode cannot be null");
    }

    /**
     * Checks if the schemas are compatible.
     *
     * @return true if compatible, false otherwise
     */
    public boolean isCompatible() {
        return compatible;
    }

    /**
     * Gets the list of incompatibility issues.
     *
     * @return an unmodifiable list of incompatibilities
     */
    @NotNull
    public List<String> getIncompatibilities() {
        return incompatibilities;
    }

    /**
     * Gets the compatibility mode used for checking.
     *
     * @return the compatibility mode
     */
    @NotNull
    public CompatibilityMode getMode() {
        return mode;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        CompatibilityResult that = (CompatibilityResult) o;
        return compatible == that.compatible &&
                Objects.equals(incompatibilities, that.incompatibilities) &&
                mode == that.mode;
    }

    @Override
    public int hashCode() {
        return Objects.hash(compatible, incompatibilities, mode);
    }

    @Override
    public String toString() {
        return "CompatibilityResult{" +
                "compatible=" + compatible +
                ", incompatibilities=" + incompatibilities +
                ", mode=" + mode +
                '}';
    }
}
