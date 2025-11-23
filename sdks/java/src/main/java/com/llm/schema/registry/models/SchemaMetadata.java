package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.*;

/**
 * Metadata for a schema.
 *
 * <p>This class contains optional metadata that can be associated with a schema,
 * including description, tags, owner information, and custom key-value pairs.
 *
 * @since 1.0.0
 */
@JsonInclude(JsonInclude.Include.NON_NULL)
public final class SchemaMetadata {
    @JsonProperty("description")
    private final String description;

    @JsonProperty("tags")
    private final List<String> tags;

    @JsonProperty("owner")
    private final String owner;

    @JsonProperty("custom")
    private final Map<String, Object> custom;

    /**
     * Private constructor for builder pattern.
     */
    private SchemaMetadata(Builder builder) {
        this.description = builder.description;
        this.tags = builder.tags != null ? Collections.unmodifiableList(new ArrayList<>(builder.tags)) : Collections.emptyList();
        this.owner = builder.owner;
        this.custom = builder.custom != null ? Collections.unmodifiableMap(new HashMap<>(builder.custom)) : Collections.emptyMap();
    }

    /**
     * Gets the schema description.
     *
     * @return the description, or null if not set
     */
    @Nullable
    public String getDescription() {
        return description;
    }

    /**
     * Gets the tags associated with this schema.
     *
     * @return an unmodifiable list of tags
     */
    @NotNull
    public List<String> getTags() {
        return tags;
    }

    /**
     * Gets the owner of this schema.
     *
     * @return the owner, or null if not set
     */
    @Nullable
    public String getOwner() {
        return owner;
    }

    /**
     * Gets custom metadata.
     *
     * @return an unmodifiable map of custom metadata
     */
    @NotNull
    public Map<String, Object> getCustom() {
        return custom;
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
     * Builder for SchemaMetadata.
     */
    public static final class Builder {
        private String description;
        private List<String> tags;
        private String owner;
        private Map<String, Object> custom;

        private Builder() {
        }

        /**
         * Sets the description.
         *
         * @param description the description
         * @return this builder
         */
        @NotNull
        public Builder description(@Nullable String description) {
            this.description = description;
            return this;
        }

        /**
         * Sets the tags.
         *
         * @param tags the tags
         * @return this builder
         */
        @NotNull
        public Builder tags(@Nullable List<String> tags) {
            this.tags = tags;
            return this;
        }

        /**
         * Adds a tag.
         *
         * @param tag the tag to add
         * @return this builder
         */
        @NotNull
        public Builder addTag(@NotNull String tag) {
            if (this.tags == null) {
                this.tags = new ArrayList<>();
            }
            this.tags.add(Objects.requireNonNull(tag, "tag cannot be null"));
            return this;
        }

        /**
         * Sets the owner.
         *
         * @param owner the owner
         * @return this builder
         */
        @NotNull
        public Builder owner(@Nullable String owner) {
            this.owner = owner;
            return this;
        }

        /**
         * Sets custom metadata.
         *
         * @param custom the custom metadata map
         * @return this builder
         */
        @NotNull
        public Builder custom(@Nullable Map<String, Object> custom) {
            this.custom = custom;
            return this;
        }

        /**
         * Adds a custom metadata entry.
         *
         * @param key   the key
         * @param value the value
         * @return this builder
         */
        @NotNull
        public Builder addCustom(@NotNull String key, @NotNull Object value) {
            if (this.custom == null) {
                this.custom = new HashMap<>();
            }
            this.custom.put(
                    Objects.requireNonNull(key, "key cannot be null"),
                    Objects.requireNonNull(value, "value cannot be null")
            );
            return this;
        }

        /**
         * Builds the SchemaMetadata instance.
         *
         * @return a new SchemaMetadata
         */
        @NotNull
        public SchemaMetadata build() {
            return new SchemaMetadata(this);
        }
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        SchemaMetadata that = (SchemaMetadata) o;
        return Objects.equals(description, that.description) &&
                Objects.equals(tags, that.tags) &&
                Objects.equals(owner, that.owner) &&
                Objects.equals(custom, that.custom);
    }

    @Override
    public int hashCode() {
        return Objects.hash(description, tags, owner, custom);
    }

    @Override
    public String toString() {
        return "SchemaMetadata{" +
                "description='" + description + '\'' +
                ", tags=" + tags +
                ", owner='" + owner + '\'' +
                ", custom=" + custom +
                '}';
    }
}
