package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.Objects;
import java.util.regex.Pattern;

/**
 * Schema definition for the LLM Schema Registry.
 *
 * <p>This class represents a schema with namespace, name, version, format, and content.
 * Use the {@link Builder} to construct instances with fluent API.
 *
 * <p>Example usage:
 * <pre>{@code
 * Schema schema = Schema.builder()
 *     .namespace("telemetry")
 *     .name("InferenceEvent")
 *     .version("1.0.0")
 *     .format(SchemaFormat.JSON_SCHEMA)
 *     .content("{\"type\": \"object\"}")
 *     .metadata(SchemaMetadata.builder()
 *         .description("Inference event schema")
 *         .addTag("ml")
 *         .build())
 *     .build();
 * }</pre>
 *
 * @since 1.0.0
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public final class Schema {
    private static final Pattern SEMVER_PATTERN = Pattern.compile("^\\d+\\.\\d+\\.\\d+$");

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

    /**
     * Private constructor for builder pattern.
     */
    private Schema(Builder builder) {
        this.namespace = Objects.requireNonNull(builder.namespace, "namespace cannot be null");
        this.name = Objects.requireNonNull(builder.name, "name cannot be null");
        this.version = validateVersion(builder.version);
        this.format = Objects.requireNonNull(builder.format, "format cannot be null");
        this.content = Objects.requireNonNull(builder.content, "content cannot be null");
        this.metadata = builder.metadata;
    }

    /**
     * Validates the version follows semantic versioning.
     *
     * @param version the version to validate
     * @return the validated version
     * @throws IllegalArgumentException if version is invalid
     */
    private static String validateVersion(String version) {
        Objects.requireNonNull(version, "version cannot be null");
        if (!SEMVER_PATTERN.matcher(version).matches()) {
            throw new IllegalArgumentException(
                    "Version must follow semantic versioning format (major.minor.patch): " + version
            );
        }
        return version;
    }

    /**
     * Gets the schema namespace.
     *
     * @return the namespace
     */
    @NotNull
    public String getNamespace() {
        return namespace;
    }

    /**
     * Gets the schema name.
     *
     * @return the name
     */
    @NotNull
    public String getName() {
        return name;
    }

    /**
     * Gets the schema version.
     *
     * @return the version
     */
    @NotNull
    public String getVersion() {
        return version;
    }

    /**
     * Gets the schema format.
     *
     * @return the format
     */
    @NotNull
    public SchemaFormat getFormat() {
        return format;
    }

    /**
     * Gets the schema content.
     *
     * @return the content
     */
    @NotNull
    public String getContent() {
        return content;
    }

    /**
     * Gets the schema metadata.
     *
     * @return the metadata, or null if not set
     */
    @Nullable
    public SchemaMetadata getMetadata() {
        return metadata;
    }

    /**
     * Creates a new builder instance.
     *
     * @return a new Builder
     */
    @NotNull
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Builder for Schema instances.
     *
     * <p>Provides a fluent API for constructing Schema objects with validation.
     */
    public static final class Builder {
        private String namespace;
        private String name;
        private String version;
        private SchemaFormat format;
        private String content;
        private SchemaMetadata metadata;

        private Builder() {
        }

        /**
         * Sets the namespace.
         *
         * @param namespace the namespace (e.g., "telemetry")
         * @return this builder
         */
        @NotNull
        public Builder namespace(@NotNull String namespace) {
            this.namespace = namespace;
            return this;
        }

        /**
         * Sets the name.
         *
         * @param name the name (e.g., "InferenceEvent")
         * @return this builder
         */
        @NotNull
        public Builder name(@NotNull String name) {
            this.name = name;
            return this;
        }

        /**
         * Sets the version.
         *
         * @param version the version in semantic versioning format (e.g., "1.0.0")
         * @return this builder
         */
        @NotNull
        public Builder version(@NotNull String version) {
            this.version = version;
            return this;
        }

        /**
         * Sets the format.
         *
         * @param format the schema format
         * @return this builder
         */
        @NotNull
        public Builder format(@NotNull SchemaFormat format) {
            this.format = format;
            return this;
        }

        /**
         * Sets the content.
         *
         * @param content the schema content (JSON/Avro/Protobuf)
         * @return this builder
         */
        @NotNull
        public Builder content(@NotNull String content) {
            this.content = content;
            return this;
        }

        /**
         * Sets the metadata.
         *
         * @param metadata the schema metadata
         * @return this builder
         */
        @NotNull
        public Builder metadata(@Nullable SchemaMetadata metadata) {
            this.metadata = metadata;
            return this;
        }

        /**
         * Builds the Schema instance.
         *
         * @return a new Schema
         * @throws NullPointerException     if required fields are null
         * @throws IllegalArgumentException if version format is invalid
         */
        @NotNull
        public Schema build() {
            return new Schema(this);
        }
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Schema schema = (Schema) o;
        return Objects.equals(namespace, schema.namespace) &&
                Objects.equals(name, schema.name) &&
                Objects.equals(version, schema.version) &&
                format == schema.format &&
                Objects.equals(content, schema.content) &&
                Objects.equals(metadata, schema.metadata);
    }

    @Override
    public int hashCode() {
        return Objects.hash(namespace, name, version, format, content, metadata);
    }

    @Override
    public String toString() {
        return "Schema{" +
                "namespace='" + namespace + '\'' +
                ", name='" + name + '\'' +
                ", version='" + version + '\'' +
                ", format=" + format +
                ", content='" + content + '\'' +
                ", metadata=" + metadata +
                '}';
    }
}
