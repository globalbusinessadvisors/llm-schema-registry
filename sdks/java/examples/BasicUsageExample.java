import com.llm.schema.registry.client.SchemaRegistryClient;
import com.llm.schema.registry.models.*;

import java.time.Duration;
import java.util.concurrent.CompletableFuture;

/**
 * Basic usage examples for the LLM Schema Registry Java SDK.
 *
 * <p>This example demonstrates:
 * <ul>
 *     <li>Creating a client with builder pattern</li>
 *     <li>Registering schemas</li>
 *     <li>Retrieving schemas</li>
 *     <li>Validating data</li>
 *     <li>Checking compatibility</li>
 *     <li>Using CompletableFuture for async operations</li>
 * </ul>
 */
public class BasicUsageExample {

    public static void main(String[] args) {
        // Create client with builder pattern
        try (SchemaRegistryClient client = SchemaRegistryClient.builder()
                .baseUrl("http://localhost:8080")
                .apiKey("your-api-key")
                .timeout(Duration.ofSeconds(30))
                .maxRetries(3)
                .cacheTTL(Duration.ofMinutes(5))
                .cacheMaxSize(1000)
                .build()) {

            // Example 1: Register a schema
            registerSchemaExample(client);

            // Example 2: Retrieve schema by ID
            getSchemaExample(client);

            // Example 3: Validate data
            validateDataExample(client);

            // Example 4: Check compatibility
            checkCompatibilityExample(client);

            // Example 5: Search schemas
            searchSchemasExample(client);

            // Example 6: Error handling
            errorHandlingExample(client);

            // Wait for all async operations to complete
            try {
                Thread.sleep(2000);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }

        } catch (Exception e) {
            System.err.println("Error: " + e.getMessage());
            e.printStackTrace();
        }
    }

    /**
     * Example 1: Register a new schema.
     */
    private static void registerSchemaExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 1: Register Schema ===");

        // Build schema with fluent API
        Schema schema = Schema.builder()
                .namespace("telemetry")
                .name("InferenceEvent")
                .version("1.0.0")
                .format(SchemaFormat.JSON_SCHEMA)
                .content("{\"type\": \"object\", \"properties\": {\"model\": {\"type\": \"string\"}}}")
                .metadata(SchemaMetadata.builder()
                        .description("Schema for inference events")
                        .addTag("ml")
                        .addTag("telemetry")
                        .owner("ml-team")
                        .addCustom("priority", "high")
                        .build())
                .build();

        // Register schema asynchronously
        client.registerSchema(schema)
                .thenAccept(response -> {
                    System.out.println("Schema registered successfully!");
                    System.out.println("  Schema ID: " + response.getSchemaId());
                    System.out.println("  Version: " + response.getVersion());
                    System.out.println("  Created At: " + response.getCreatedAt());
                })
                .exceptionally(error -> {
                    System.err.println("Failed to register schema: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 2: Retrieve a schema by ID.
     */
    private static void getSchemaExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 2: Get Schema ===");

        String schemaId = "550e8400-e29b-41d4-a716-446655440000";

        client.getSchema(schemaId)
                .thenAccept(schema -> {
                    System.out.println("Schema retrieved successfully!");
                    System.out.println("  Namespace: " + schema.getNamespace());
                    System.out.println("  Name: " + schema.getName());
                    System.out.println("  Version: " + schema.getVersion());
                    System.out.println("  Format: " + schema.getFormat());
                })
                .exceptionally(error -> {
                    System.err.println("Failed to get schema: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 3: Validate data against a schema.
     */
    private static void validateDataExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 3: Validate Data ===");

        String schemaId = "550e8400-e29b-41d4-a716-446655440000";
        String data = "{\"model\": \"gpt-4\", \"timestamp\": \"2025-01-01T00:00:00Z\"}";

        client.validateData(schemaId, data)
                .thenAccept(result -> {
                    if (result.isValid()) {
                        System.out.println("Data is valid!");
                    } else {
                        System.out.println("Data is invalid. Errors:");
                        result.getErrors().forEach(error -> System.out.println("  - " + error));
                    }
                })
                .exceptionally(error -> {
                    System.err.println("Failed to validate data: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 4: Check schema compatibility.
     */
    private static void checkCompatibilityExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 4: Check Compatibility ===");

        String schemaId = "550e8400-e29b-41d4-a716-446655440000";
        String newSchemaContent = "{\"type\": \"object\", \"properties\": " +
                "{\"model\": {\"type\": \"string\"}, \"version\": {\"type\": \"string\"}}}";

        client.checkCompatibility(schemaId, newSchemaContent, CompatibilityMode.BACKWARD)
                .thenAccept(result -> {
                    if (result.isCompatible()) {
                        System.out.println("Schema is compatible!");
                    } else {
                        System.out.println("Schema is incompatible. Issues:");
                        result.getIncompatibilities().forEach(issue -> System.out.println("  - " + issue));
                    }
                    System.out.println("  Mode: " + result.getMode());
                })
                .exceptionally(error -> {
                    System.err.println("Failed to check compatibility: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 5: Search for schemas.
     */
    private static void searchSchemasExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 5: Search Schemas ===");

        client.searchSchemas("inference", 10, 0)
                .thenAccept(results -> {
                    System.out.println("Found " + results.size() + " schemas:");
                    results.forEach(result -> {
                        System.out.println("  - " + result.getNamespace() + "." + result.getName() +
                                " v" + result.getVersion() + " (score: " + result.getScore() + ")");
                    });
                })
                .exceptionally(error -> {
                    System.err.println("Failed to search schemas: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 6: Error handling with CompletableFuture.
     */
    private static void errorHandlingExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 6: Error Handling ===");

        // Try to get a non-existent schema
        client.getSchema("non-existent-id")
                .thenAccept(schema -> {
                    System.out.println("Schema found: " + schema.getName());
                })
                .exceptionally(error -> {
                    System.err.println("Expected error occurred: " + error.getMessage());

                    // Handle specific exception types
                    Throwable cause = error.getCause();
                    if (cause instanceof com.llm.schema.registry.exceptions.SchemaNotFoundException) {
                        System.err.println("Schema not found (404)");
                    } else if (cause instanceof com.llm.schema.registry.exceptions.AuthenticationException) {
                        System.err.println("Authentication failed (401)");
                    } else if (cause instanceof com.llm.schema.registry.exceptions.ServerException) {
                        System.err.println("Server error (5xx)");
                    }

                    return null;
                });
    }

    /**
     * Example 7: Chaining async operations.
     */
    private static void chainingExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 7: Chaining Operations ===");

        Schema schema = Schema.builder()
                .namespace("telemetry")
                .name("UserEvent")
                .version("1.0.0")
                .format(SchemaFormat.JSON_SCHEMA)
                .content("{\"type\": \"object\"}")
                .build();

        // Chain operations: register -> get -> validate
        client.registerSchema(schema)
                .thenCompose(registerResponse -> {
                    System.out.println("Schema registered with ID: " + registerResponse.getSchemaId());
                    return client.getSchema(registerResponse.getSchemaId());
                })
                .thenCompose(schemaResponse -> {
                    System.out.println("Schema retrieved: " + schemaResponse.getName());
                    return client.validateData(schemaResponse.getSchemaId(), "{\"test\": \"data\"}");
                })
                .thenAccept(validateResponse -> {
                    System.out.println("Validation result: " + (validateResponse.isValid() ? "valid" : "invalid"));
                })
                .exceptionally(error -> {
                    System.err.println("Operation chain failed: " + error.getMessage());
                    return null;
                });
    }

    /**
     * Example 8: Parallel operations with CompletableFuture.allOf.
     */
    private static void parallelExample(SchemaRegistryClient client) {
        System.out.println("\n=== Example 8: Parallel Operations ===");

        CompletableFuture<GetSchemaResponse> future1 = client.getSchema("id-1");
        CompletableFuture<GetSchemaResponse> future2 = client.getSchema("id-2");
        CompletableFuture<GetSchemaResponse> future3 = client.getSchema("id-3");

        CompletableFuture.allOf(future1, future2, future3)
                .thenRun(() -> {
                    System.out.println("All schemas retrieved successfully!");
                    try {
                        System.out.println("  Schema 1: " + future1.get().getName());
                        System.out.println("  Schema 2: " + future2.get().getName());
                        System.out.println("  Schema 3: " + future3.get().getName());
                    } catch (Exception e) {
                        System.err.println("Error getting results: " + e.getMessage());
                    }
                })
                .exceptionally(error -> {
                    System.err.println("One or more operations failed: " + error.getMessage());
                    return null;
                });
    }
}
