package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.time.Instant;
import java.util.Objects;

/**
 * Response from schema retrieval operation.
 *
 * @since 1.0.0
 */
public final class GetSchemaResponse {
    @JsonProperty("schema_id")
    private final String schemaId;

    @JsonProperty("namespace")
    private final String namespace;

    @JsonProperty("name")
    private final String name;

    @JsonProperty("version")
    private final String version;

    @JsonProperty("format")
    private final SchemaFormat format;

    @JsonProperty("content")
    private final String content;

    @JsonProperty("metadata")
    private final SchemaMetadata metadata;

    @JsonProperty("created_at")
    private final Instant createdAt;

    @JsonProperty("updated_at")
    private final Instant updatedAt;

    /**
     * Creates a new GetSchemaResponse.
     *
     * @param schemaId  the schema ID
     * @param namespace the namespace
     * @param name      the name
     * @param version   the version
     * @param format    the format
     * @param content   the content
     * @param metadata  the metadata (optional)
     * @param createdAt the creation timestamp
     * @param updatedAt the update timestamp
     */
    @JsonCreator
    public GetSchemaResponse(
            @JsonProperty("schema_id") String schemaId,
            @JsonProperty("namespace") String namespace,
            @JsonProperty("name") String name,
            @JsonProperty("version") String version,
            @JsonProperty("format") SchemaFormat format,
            @JsonProperty("content") String content,
            @JsonProperty("metadata") SchemaMetadata metadata,
            @JsonProperty("created_at") Instant createdAt,
            @JsonProperty("updated_at") Instant updatedAt) {
        this.schemaId = Objects.requireNonNull(schemaId, "schemaId cannot be null");
        this.namespace = Objects.requireNonNull(namespace, "namespace cannot be null");
        this.name = Objects.requireNonNull(name, "name cannot be null");
        this.version = Objects.requireNonNull(version, "version cannot be null");
        this.format = Objects.requireNonNull(format, "format cannot be null");
        this.content = Objects.requireNonNull(content, "content cannot be null");
        this.metadata = metadata;
        this.createdAt = Objects.requireNonNull(createdAt, "createdAt cannot be null");
        this.updatedAt = Objects.requireNonNull(updatedAt, "updatedAt cannot be null");
    }

    @NotNull
    public String getSchemaId() {
        return schemaId;
    }

    @NotNull
    public String getNamespace() {
        return namespace;
    }

    @NotNull
    public String getName() {
        return name;
    }

    @NotNull
    public String getVersion() {
        return version;
    }

    @NotNull
    public SchemaFormat getFormat() {
        return format;
    }

    @NotNull
    public String getContent() {
        return content;
    }

    @Nullable
    public SchemaMetadata getMetadata() {
        return metadata;
    }

    @NotNull
    public Instant getCreatedAt() {
        return createdAt;
    }

    @NotNull
    public Instant getUpdatedAt() {
        return updatedAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        GetSchemaResponse that = (GetSchemaResponse) o;
        return Objects.equals(schemaId, that.schemaId) &&
                Objects.equals(namespace, that.namespace) &&
                Objects.equals(name, that.name) &&
                Objects.equals(version, that.version) &&
                format == that.format &&
                Objects.equals(content, that.content) &&
                Objects.equals(metadata, that.metadata) &&
                Objects.equals(createdAt, that.createdAt) &&
                Objects.equals(updatedAt, that.updatedAt);
    }

    @Override
    public int hashCode() {
        return Objects.hash(schemaId, namespace, name, version, format, content, metadata, createdAt, updatedAt);
    }

    @Override
    public String toString() {
        return "GetSchemaResponse{" +
                "schemaId='" + schemaId + '\'' +
                ", namespace='" + namespace + '\'' +
                ", name='" + name + '\'' +
                ", version='" + version + '\'' +
                ", format=" + format +
                ", content='" + content + '\'' +
                ", metadata=" + metadata +
                ", createdAt=" + createdAt +
                ", updatedAt=" + updatedAt +
                '}';
    }
}
