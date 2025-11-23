package com.llm.schema.registry.client;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import com.llm.schema.registry.cache.SimpleCache;
import com.llm.schema.registry.exceptions.*;
import com.llm.schema.registry.models.*;
import okhttp3.*;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.Closeable;
import java.io.IOException;
import java.time.Duration;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

/**
 * Production-ready client for the LLM Schema Registry.
 *
 * <p>This client provides asynchronous operations using {@link CompletableFuture}
 * for non-blocking schema registry operations.
 *
 * <p><strong>Features:</strong>
 * <ul>
 *     <li>Async/CompletableFuture support for all operations</li>
 *     <li>Automatic retries with exponential backoff</li>
 *     <li>In-memory caching with TTL (5 minutes default)</li>
 *     <li>Comprehensive error handling with specific exceptions</li>
 *     <li>Type-safe schema operations</li>
 *     <li>Thread-safe implementation</li>
 * </ul>
 *
 * <p><strong>Example usage:</strong>
 * <pre>{@code
 * SchemaRegistryClient client = SchemaRegistryClient.builder()
 *     .baseUrl("http://localhost:8080")
 *     .apiKey("your-api-key")
 *     .timeout(Duration.ofSeconds(30))
 *     .build();
 *
 * Schema schema = Schema.builder()
 *     .namespace("telemetry")
 *     .name("InferenceEvent")
 *     .version("1.0.0")
 *     .format(SchemaFormat.JSON_SCHEMA)
 *     .content("{\"type\": \"object\"}")
 *     .build();
 *
 * client.registerSchema(schema)
 *     .thenAccept(response -> {
 *         System.out.println("Schema ID: " + response.getSchemaId());
 *     })
 *     .exceptionally(error -> {
 *         System.err.println("Error: " + error.getMessage());
 *         return null;
 *     });
 *
 * // Don't forget to close the client when done
 * client.close();
 * }</pre>
 *
 * @since 1.0.0
 */
public final class SchemaRegistryClient implements Closeable {
    private static final Logger logger = LoggerFactory.getLogger(SchemaRegistryClient.class);
    private static final MediaType JSON = MediaType.get("application/json; charset=utf-8");

    private final String baseUrl;
    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;
    private final SimpleCache<String, Object> cache;
    private final int maxRetries;

    /**
     * Private constructor for builder pattern.
     */
    private SchemaRegistryClient(Builder builder) {
        this.baseUrl = builder.baseUrl.endsWith("/") ? builder.baseUrl.substring(0, builder.baseUrl.length() - 1) : builder.baseUrl;
        this.maxRetries = builder.maxRetries;

        // Initialize ObjectMapper with Java 8 time support
        this.objectMapper = new ObjectMapper()
                .registerModule(new JavaTimeModule())
                .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

        // Initialize cache
        this.cache = new SimpleCache<>(builder.cacheTTL.toMillis(), builder.cacheMaxSize);

        // Build OkHttpClient
        OkHttpClient.Builder httpClientBuilder = new OkHttpClient.Builder()
                .connectTimeout(builder.timeout)
                .readTimeout(builder.timeout)
                .writeTimeout(builder.timeout)
                .retryOnConnectionFailure(true);

        // Add authentication if API key is provided
        if (builder.apiKey != null) {
            httpClientBuilder.addInterceptor(chain -> {
                Request original = chain.request();
                Request authenticated = original.newBuilder()
                        .header("Authorization", "Bearer " + builder.apiKey)
                        .build();
                return chain.proceed(authenticated);
            });
        }

        this.httpClient = httpClientBuilder.build();
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
     * Registers a new schema.
     *
     * @param schema the schema to register
     * @return a CompletableFuture that completes with the registration response
     */
    @NotNull
    public CompletableFuture<RegisterSchemaResponse> registerSchema(@NotNull Schema schema) {
        Objects.requireNonNull(schema, "schema cannot be null");

        logger.info("Registering schema: {}.{} v{}", schema.getNamespace(), schema.getName(), schema.getVersion());

        return CompletableFuture.supplyAsync(() -> {
            try {
                String json = objectMapper.writeValueAsString(schema);
                RequestBody body = RequestBody.create(json, JSON);
                Request request = new Request.Builder()
                        .url(baseUrl + "/api/v1/schemas")
                        .post(body)
                        .build();

                RegisterSchemaResponse response = executeWithRetry(request, RegisterSchemaResponse.class);

                // Invalidate cache for this schema
                String cacheKey = schema.getNamespace() + "." + schema.getName();
                cache.remove(cacheKey);

                logger.info("Registered schema with ID: {}", response.getSchemaId());
                return response;
            } catch (IOException e) {
                throw new RuntimeException("Failed to serialize schema", e);
            }
        });
    }

    /**
     * Gets a schema by ID.
     *
     * @param schemaId the schema ID
     * @return a CompletableFuture that completes with the schema
     */
    @NotNull
    public CompletableFuture<GetSchemaResponse> getSchema(@NotNull String schemaId) {
        return getSchema(schemaId, true);
    }

    /**
     * Gets a schema by ID with optional caching.
     *
     * @param schemaId the schema ID
     * @param useCache whether to use cached result
     * @return a CompletableFuture that completes with the schema
     */
    @NotNull
    public CompletableFuture<GetSchemaResponse> getSchema(@NotNull String schemaId, boolean useCache) {
        Objects.requireNonNull(schemaId, "schemaId cannot be null");

        // Check cache first
        if (useCache) {
            String cacheKey = "schema:" + schemaId;
            GetSchemaResponse cached = (GetSchemaResponse) cache.get(cacheKey);
            if (cached != null) {
                logger.debug("Cache hit for schema ID: {}", schemaId);
                return CompletableFuture.completedFuture(cached);
            }
        }

        logger.info("Fetching schema: {}", schemaId);

        return CompletableFuture.supplyAsync(() -> {
            Request request = new Request.Builder()
                    .url(baseUrl + "/api/v1/schemas/" + schemaId)
                    .get()
                    .build();

            GetSchemaResponse response = executeWithRetry(request, GetSchemaResponse.class);

            // Cache the result
            if (useCache) {
                cache.put("schema:" + schemaId, response);
            }

            return response;
        });
    }

    /**
     * Gets a schema by namespace, name, and version.
     *
     * @param namespace the schema namespace
     * @param name      the schema name
     * @param version   the schema version
     * @return a CompletableFuture that completes with the schema
     */
    @NotNull
    public CompletableFuture<GetSchemaResponse> getSchemaByVersion(
            @NotNull String namespace,
            @NotNull String name,
            @NotNull String version) {
        Objects.requireNonNull(namespace, "namespace cannot be null");
        Objects.requireNonNull(name, "name cannot be null");
        Objects.requireNonNull(version, "version cannot be null");

        // Check cache
        String cacheKey = "schema:" + namespace + "." + name + ":" + version;
        GetSchemaResponse cached = (GetSchemaResponse) cache.get(cacheKey);
        if (cached != null) {
            logger.debug("Cache hit for schema: {}.{} v{}", namespace, name, version);
            return CompletableFuture.completedFuture(cached);
        }

        logger.info("Fetching schema: {}.{} v{}", namespace, name, version);

        return CompletableFuture.supplyAsync(() -> {
            Request request = new Request.Builder()
                    .url(String.format("%s/api/v1/schemas/%s/%s/versions/%s", baseUrl, namespace, name, version))
                    .get()
                    .build();

            GetSchemaResponse response = executeWithRetry(request, GetSchemaResponse.class);

            // Cache the result
            cache.put(cacheKey, response);

            return response;
        });
    }

    /**
     * Validates data against a schema.
     *
     * @param schemaId the schema ID to validate against
     * @param data     the data to validate (JSON string or Map)
     * @return a CompletableFuture that completes with the validation result
     */
    @NotNull
    public CompletableFuture<ValidateResponse> validateData(@NotNull String schemaId, @NotNull Object data) {
        Objects.requireNonNull(schemaId, "schemaId cannot be null");
        Objects.requireNonNull(data, "data cannot be null");

        logger.info("Validating data against schema: {}", schemaId);

        return CompletableFuture.supplyAsync(() -> {
            try {
                String dataJson = data instanceof String ? (String) data : objectMapper.writeValueAsString(data);

                Map<String, Object> payload = new HashMap<>();
                payload.put("schema_id", schemaId);
                payload.put("data", dataJson);

                String json = objectMapper.writeValueAsString(payload);
                RequestBody body = RequestBody.create(json, JSON);
                Request request = new Request.Builder()
                        .url(baseUrl + "/api/v1/validate")
                        .post(body)
                        .build();

                return executeWithRetry(request, ValidateResponse.class);
            } catch (IOException e) {
                throw new RuntimeException("Failed to serialize data", e);
            }
        });
    }

    /**
     * Checks if a new schema is compatible with an existing schema.
     *
     * @param schemaId         the existing schema ID
     * @param newSchemaContent the new schema content
     * @param mode             the compatibility mode
     * @return a CompletableFuture that completes with the compatibility result
     */
    @NotNull
    public CompletableFuture<CompatibilityResult> checkCompatibility(
            @NotNull String schemaId,
            @NotNull String newSchemaContent,
            @NotNull CompatibilityMode mode) {
        Objects.requireNonNull(schemaId, "schemaId cannot be null");
        Objects.requireNonNull(newSchemaContent, "newSchemaContent cannot be null");
        Objects.requireNonNull(mode, "mode cannot be null");

        logger.info("Checking compatibility for schema: {} (mode: {})", schemaId, mode);

        return CompletableFuture.supplyAsync(() -> {
            try {
                Map<String, Object> payload = new HashMap<>();
                payload.put("schema_id", schemaId);
                payload.put("new_schema", newSchemaContent);
                payload.put("mode", mode.getValue());

                String json = objectMapper.writeValueAsString(payload);
                RequestBody body = RequestBody.create(json, JSON);
                Request request = new Request.Builder()
                        .url(baseUrl + "/api/v1/compatibility/check")
                        .post(body)
                        .build();

                return executeWithRetry(request, CompatibilityResult.class);
            } catch (IOException e) {
                throw new RuntimeException("Failed to serialize compatibility check request", e);
            }
        });
    }

    /**
     * Searches for schemas.
     *
     * @param query  the search query
     * @param limit  the maximum number of results
     * @param offset the result offset for pagination
     * @return a CompletableFuture that completes with the search results
     */
    @NotNull
    public CompletableFuture<List<SearchResult>> searchSchemas(
            @NotNull String query,
            int limit,
            int offset) {
        Objects.requireNonNull(query, "query cannot be null");

        logger.info("Searching schemas: query='{}', limit={}, offset={}", query, limit, offset);

        return CompletableFuture.supplyAsync(() -> {
            HttpUrl url = Objects.requireNonNull(HttpUrl.parse(baseUrl + "/api/v1/search"))
                    .newBuilder()
                    .addQueryParameter("q", query)
                    .addQueryParameter("limit", String.valueOf(limit))
                    .addQueryParameter("offset", String.valueOf(offset))
                    .build();

            Request request = new Request.Builder()
                    .url(url)
                    .get()
                    .build();

            return executeWithRetry(request, new TypeReference<List<SearchResult>>() {
            });
        });
    }

    /**
     * Lists all versions of a schema.
     *
     * @param namespace the schema namespace
     * @param name      the schema name
     * @return a CompletableFuture that completes with the list of versions
     */
    @NotNull
    public CompletableFuture<List<SchemaVersion>> listVersions(
            @NotNull String namespace,
            @NotNull String name) {
        Objects.requireNonNull(namespace, "namespace cannot be null");
        Objects.requireNonNull(name, "name cannot be null");

        logger.info("Listing versions for schema: {}.{}", namespace, name);

        return CompletableFuture.supplyAsync(() -> {
            Request request = new Request.Builder()
                    .url(String.format("%s/api/v1/schemas/%s/%s/versions", baseUrl, namespace, name))
                    .get()
                    .build();

            return executeWithRetry(request, new TypeReference<List<SchemaVersion>>() {
            });
        });
    }

    /**
     * Deletes a schema.
     *
     * @param schemaId the schema ID to delete
     * @return a CompletableFuture that completes when deletion is successful
     */
    @NotNull
    public CompletableFuture<Void> deleteSchema(@NotNull String schemaId) {
        Objects.requireNonNull(schemaId, "schemaId cannot be null");

        logger.info("Deleting schema: {}", schemaId);

        return CompletableFuture.runAsync(() -> {
            Request request = new Request.Builder()
                    .url(baseUrl + "/api/v1/schemas/" + schemaId)
                    .delete()
                    .build();

            executeWithRetry(request, Void.class);

            // Invalidate cache
            cache.remove("schema:" + schemaId);

            logger.info("Deleted schema: {}", schemaId);
        });
    }

    /**
     * Performs a health check on the schema registry.
     *
     * @return a CompletableFuture that completes with the health status
     */
    @NotNull
    public CompletableFuture<Map<String, Object>> healthCheck() {
        logger.info("Performing health check");

        return CompletableFuture.supplyAsync(() -> {
            Request request = new Request.Builder()
                    .url(baseUrl + "/health")
                    .get()
                    .build();

            return executeWithRetry(request, new TypeReference<Map<String, Object>>() {
            });
        });
    }

    /**
     * Clears the in-memory cache.
     */
    public void clearCache() {
        cache.clear();
        logger.info("Cache cleared");
    }

    /**
     * Executes a request with retry logic.
     *
     * @param request the HTTP request
     * @param clazz   the response class
     * @param <T>     the response type
     * @return the response object
     * @throws SchemaRegistryException if the request fails
     */
    private <T> T executeWithRetry(Request request, Class<T> clazz) {
        return executeWithRetry(request, new TypeReference<T>() {
            @Override
            public Class<T> getType() {
                return clazz;
            }
        });
    }

    /**
     * Executes a request with retry logic.
     *
     * @param request       the HTTP request
     * @param typeReference the response type reference
     * @param <T>           the response type
     * @return the response object
     * @throws SchemaRegistryException if the request fails
     */
    private <T> T executeWithRetry(Request request, TypeReference<T> typeReference) {
        int attempt = 0;
        Exception lastException = null;

        while (attempt <= maxRetries) {
            try {
                try (Response response = httpClient.newCall(request).execute()) {
                    return handleResponse(response, typeReference);
                }
            } catch (IOException e) {
                lastException = e;
                attempt++;

                if (attempt <= maxRetries) {
                    long delayMs = (long) Math.min(1000 * Math.pow(2, attempt - 1), 10000);
                    logger.warn("Request failed (attempt {}/{}), retrying in {}ms: {}",
                            attempt, maxRetries + 1, delayMs, e.getMessage());

                    try {
                        Thread.sleep(delayMs);
                    } catch (InterruptedException ie) {
                        Thread.currentThread().interrupt();
                        throw new RuntimeException("Interrupted during retry", ie);
                    }
                } else {
                    break;
                }
            } catch (SchemaRegistryException e) {
                // Retry on server errors (5xx), don't retry on client errors (4xx)
                if (e instanceof ServerException) {
                    lastException = e;
                    attempt++;

                    if (attempt <= maxRetries) {
                        long delayMs = (long) Math.min(1000 * Math.pow(2, attempt - 1), 10000);
                        logger.warn("Request failed (attempt {}/{}), retrying in {}ms: {}",
                                attempt, maxRetries + 1, delayMs, e.getMessage());

                        try {
                            Thread.sleep(delayMs);
                        } catch (InterruptedException ie) {
                            Thread.currentThread().interrupt();
                            throw new RuntimeException("Interrupted during retry", ie);
                        }
                    } else {
                        break;
                    }
                } else {
                    // Client error (4xx) - don't retry
                    throw new RuntimeException(e);
                }
            }
        }

        if (lastException instanceof SchemaRegistryException) {
            throw new RuntimeException(lastException);
        }
        throw new RuntimeException("Request failed after " + (maxRetries + 1) + " attempts", lastException);
    }

    /**
     * Handles HTTP response and converts to appropriate type or exception.
     *
     * @param response      the HTTP response
     * @param typeReference the response type reference
     * @param <T>           the response type
     * @return the response object
     * @throws SchemaRegistryException if the response indicates an error
     */
    private <T> T handleResponse(Response response, TypeReference<T> typeReference) throws IOException, SchemaRegistryException {
        int statusCode = response.code();
        String responseBody = response.body() != null ? response.body().string() : "";

        if (response.isSuccessful()) {
            if (typeReference.getType() == Void.class || responseBody.isEmpty()) {
                return null;
            }
            return objectMapper.readValue(responseBody, typeReference);
        }

        // Handle error responses
        String errorMessage;
        try {
            Map<String, Object> errorData = objectMapper.readValue(responseBody, new TypeReference<Map<String, Object>>() {
            });
            errorMessage = (String) errorData.getOrDefault("message", responseBody);

            // Handle specific error types
            switch (statusCode) {
                case 401:
                    throw new AuthenticationException(errorMessage);
                case 403:
                    throw new AuthorizationException(errorMessage);
                case 404:
                    throw new SchemaNotFoundException(errorMessage, errorMessage);
                case 409:
                    @SuppressWarnings("unchecked")
                    List<String> incompatibilities = (List<String>) errorData.getOrDefault("incompatibilities", List.of(errorMessage));
                    throw new IncompatibleSchemaException(incompatibilities);
                case 429:
                    String retryAfterHeader = response.header("Retry-After");
                    Integer retryAfter = retryAfterHeader != null ? Integer.parseInt(retryAfterHeader) : null;
                    throw new RateLimitException(errorMessage, retryAfter);
                default:
                    if (statusCode >= 500) {
                        throw new ServerException(errorMessage, statusCode);
                    } else {
                        throw new SchemaRegistryException(errorMessage, statusCode);
                    }
            }
        } catch (SchemaRegistryException e) {
            throw e;
        } catch (Exception e) {
            throw new SchemaRegistryException("Error parsing error response: " + responseBody, statusCode);
        }
    }

    /**
     * Closes the client and releases resources.
     */
    @Override
    public void close() {
        cache.shutdown();
        httpClient.dispatcher().executorService().shutdown();
        httpClient.connectionPool().evictAll();
        logger.info("SchemaRegistryClient closed");
    }

    /**
     * Builder for SchemaRegistryClient.
     */
    public static final class Builder {
        private String baseUrl;
        private String apiKey;
        private Duration timeout = Duration.ofSeconds(30);
        private int maxRetries = 3;
        private Duration cacheTTL = Duration.ofMinutes(5);
        private int cacheMaxSize = 1000;

        private Builder() {
        }

        /**
         * Sets the base URL of the schema registry.
         *
         * @param baseUrl the base URL (e.g., "http://localhost:8080")
         * @return this builder
         */
        @NotNull
        public Builder baseUrl(@NotNull String baseUrl) {
            this.baseUrl = Objects.requireNonNull(baseUrl, "baseUrl cannot be null");
            return this;
        }

        /**
         * Sets the API key for authentication.
         *
         * @param apiKey the API key
         * @return this builder
         */
        @NotNull
        public Builder apiKey(@Nullable String apiKey) {
            this.apiKey = apiKey;
            return this;
        }

        /**
         * Sets the request timeout.
         *
         * @param timeout the timeout duration
         * @return this builder
         */
        @NotNull
        public Builder timeout(@NotNull Duration timeout) {
            this.timeout = Objects.requireNonNull(timeout, "timeout cannot be null");
            return this;
        }

        /**
         * Sets the maximum number of retry attempts.
         *
         * @param maxRetries the maximum retries
         * @return this builder
         */
        @NotNull
        public Builder maxRetries(int maxRetries) {
            if (maxRetries < 0) {
                throw new IllegalArgumentException("maxRetries must be >= 0");
            }
            this.maxRetries = maxRetries;
            return this;
        }

        /**
         * Sets the cache TTL.
         *
         * @param cacheTTL the cache time-to-live
         * @return this builder
         */
        @NotNull
        public Builder cacheTTL(@NotNull Duration cacheTTL) {
            this.cacheTTL = Objects.requireNonNull(cacheTTL, "cacheTTL cannot be null");
            return this;
        }

        /**
         * Sets the maximum cache size.
         *
         * @param cacheMaxSize the maximum number of cached items
         * @return this builder
         */
        @NotNull
        public Builder cacheMaxSize(int cacheMaxSize) {
            if (cacheMaxSize <= 0) {
                throw new IllegalArgumentException("cacheMaxSize must be > 0");
            }
            this.cacheMaxSize = cacheMaxSize;
            return this;
        }

        /**
         * Builds the SchemaRegistryClient instance.
         *
         * @return a new SchemaRegistryClient
         * @throws NullPointerException if required fields are null
         */
        @NotNull
        public SchemaRegistryClient build() {
            Objects.requireNonNull(baseUrl, "baseUrl must be set");
            return new SchemaRegistryClient(this);
        }
    }
}
