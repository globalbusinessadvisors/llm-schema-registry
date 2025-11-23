package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;

import java.time.Instant;
import java.util.Objects;

/**
 * Schema version information.
 *
 * @since 1.0.0
 */
public final class SchemaVersion {
    @JsonProperty("version")
    private final String version;

    @JsonProperty("schema_id")
    private final String schemaId;

    @JsonProperty("created_at")
    private final Instant createdAt;

    /**
     * Creates a new SchemaVersion.
     *
     * @param version   the version
     * @param schemaId  the schema ID
     * @param createdAt the creation timestamp
     */
    @JsonCreator
    public SchemaVersion(
            @JsonProperty("version") String version,
            @JsonProperty("schema_id") String schemaId,
            @JsonProperty("created_at") Instant createdAt) {
        this.version = Objects.requireNonNull(version, "version cannot be null");
        this.schemaId = Objects.requireNonNull(schemaId, "schemaId cannot be null");
        this.createdAt = Objects.requireNonNull(createdAt, "createdAt cannot be null");
    }

    @NotNull
    public String getVersion() {
        return version;
    }

    @NotNull
    public String getSchemaId() {
        return schemaId;
    }

    @NotNull
    public Instant getCreatedAt() {
        return createdAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        SchemaVersion that = (SchemaVersion) o;
        return Objects.equals(version, that.version) &&
                Objects.equals(schemaId, that.schemaId) &&
                Objects.equals(createdAt, that.createdAt);
    }

    @Override
    public int hashCode() {
        return Objects.hash(version, schemaId, createdAt);
    }

    @Override
    public String toString() {
        return "SchemaVersion{" +
                "version='" + version + '\'' +
                ", schemaId='" + schemaId + '\'' +
                ", createdAt=" + createdAt +
                '}';
    }
}
