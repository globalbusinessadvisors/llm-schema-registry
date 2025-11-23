package com.llm.schema.registry.models;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * Schema search result.
 *
 * @since 1.0.0
 */
public final class SearchResult {
    @JsonProperty("schema_id")
    private final String schemaId;

    @JsonProperty("namespace")
    private final String namespace;

    @JsonProperty("name")
    private final String name;

    @JsonProperty("version")
    private final String version;

    @JsonProperty("description")
    private final String description;

    @JsonProperty("tags")
    private final List<String> tags;

    @JsonProperty("score")
    private final double score;

    /**
     * Creates a new SearchResult.
     *
     * @param schemaId    the schema ID
     * @param namespace   the namespace
     * @param name        the name
     * @param version     the version
     * @param description the description (optional)
     * @param tags        the tags
     * @param score       the relevance score
     */
    @JsonCreator
    public SearchResult(
            @JsonProperty("schema_id") String schemaId,
            @JsonProperty("namespace") String namespace,
            @JsonProperty("name") String name,
            @JsonProperty("version") String version,
            @JsonProperty("description") String description,
            @JsonProperty("tags") List<String> tags,
            @JsonProperty("score") double score) {
        this.schemaId = Objects.requireNonNull(schemaId, "schemaId cannot be null");
        this.namespace = Objects.requireNonNull(namespace, "namespace cannot be null");
        this.name = Objects.requireNonNull(name, "name cannot be null");
        this.version = Objects.requireNonNull(version, "version cannot be null");
        this.description = description;
        this.tags = tags != null ? Collections.unmodifiableList(new ArrayList<>(tags)) : Collections.emptyList();
        this.score = score;
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

    @Nullable
    public String getDescription() {
        return description;
    }

    @NotNull
    public List<String> getTags() {
        return tags;
    }

    public double getScore() {
        return score;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        SearchResult that = (SearchResult) o;
        return Double.compare(that.score, score) == 0 &&
                Objects.equals(schemaId, that.schemaId) &&
                Objects.equals(namespace, that.namespace) &&
                Objects.equals(name, that.name) &&
                Objects.equals(version, that.version) &&
                Objects.equals(description, that.description) &&
                Objects.equals(tags, that.tags);
    }

    @Override
    public int hashCode() {
        return Objects.hash(schemaId, namespace, name, version, description, tags, score);
    }

    @Override
    public String toString() {
        return "SearchResult{" +
                "schemaId='" + schemaId + '\'' +
                ", namespace='" + namespace + '\'' +
                ", name='" + name + '\'' +
                ", version='" + version + '\'' +
                ", description='" + description + '\'' +
                ", tags=" + tags +
                ", score=" + score +
                '}';
    }
}
