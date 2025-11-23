package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;

import java.time.Instant;
import java.util.Objects;

/**
 * Response from schema registration operation.
 *
 * @since 1.0.0
 */
public final class RegisterSchemaResponse {
    @JsonProperty("schema_id")
    private final String schemaId;

    @JsonProperty("version")
    private final String version;

    @JsonProperty("created_at")
    private final Instant createdAt;

    /**
     * Creates a new RegisterSchemaResponse.
     *
     * @param schemaId  the unique schema ID (UUID)
     * @param version   the registered version
     * @param createdAt the creation timestamp
     */
    @JsonCreator
    public RegisterSchemaResponse(
            @JsonProperty("schema_id") String schemaId,
            @JsonProperty("version") String version,
            @JsonProperty("created_at") Instant createdAt) {
        this.schemaId = Objects.requireNonNull(schemaId, "schemaId cannot be null");
        this.version = Objects.requireNonNull(version, "version cannot be null");
        this.createdAt = Objects.requireNonNull(createdAt, "createdAt cannot be null");
    }

    /**
     * Gets the schema ID.
     *
     * @return the schema ID
     */
    @NotNull
    public String getSchemaId() {
        return schemaId;
    }

    /**
     * Gets the version.
     *
     * @return the version
     */
    @NotNull
    public String getVersion() {
        return version;
    }

    /**
     * Gets the creation timestamp.
     *
     * @return the creation timestamp
     */
    @NotNull
    public Instant getCreatedAt() {
        return createdAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        RegisterSchemaResponse that = (RegisterSchemaResponse) o;
        return Objects.equals(schemaId, that.schemaId) &&
                Objects.equals(version, that.version) &&
                Objects.equals(createdAt, that.createdAt);
    }

    @Override
    public int hashCode() {
        return Objects.hash(schemaId, version, createdAt);
    }

    @Override
    public String toString() {
        return "RegisterSchemaResponse{" +
                "schemaId='" + schemaId + '\'' +
                ", version='" + version + '\'' +
                ", createdAt=" + createdAt +
                '}';
    }
}
