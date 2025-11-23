# LLM Schema Registry Java SDK

Production-ready Java client for the LLM Schema Registry with enterprise-grade features.

## Features

- **Builder Pattern** - Fluent API for constructing complex objects
- **CompletableFuture** - Asynchronous operations for non-blocking calls
- **Automatic Retries** - Exponential backoff for transient failures
- **Smart Caching** - In-memory cache with TTL (5 minutes default)
- **Type Safety** - Strong typing with Java generics
- **Comprehensive Error Handling** - Specific exception types for different errors
- **Thread-Safe** - Safe for use in concurrent applications
- **Zero Dependencies** - Only requires OkHttp and Jackson

## Requirements

- Java 11 or higher
- Maven 3.6+ or Gradle 7+

## Installation

### Maven

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>com.llm.schema</groupId>
    <artifactId>llm-schema-registry-client</artifactId>
    <version>1.0.0</version>
</dependency>
```

### Gradle

Add to your `build.gradle`:

```groovy
implementation 'com.llm.schema:llm-schema-registry-client:1.0.0'
```

## Quick Start

```java
import com.llm.schema.registry.client.SchemaRegistryClient;
import com.llm.schema.registry.models.*;

import java.time.Duration;

public class Example {
    public static void main(String[] args) {
        // Create client with builder pattern
        try (SchemaRegistryClient client = SchemaRegistryClient.builder()
                .baseUrl("http://localhost:8080")
                .apiKey("your-api-key")
                .timeout(Duration.ofSeconds(30))
                .build()) {

            // Build and register a schema
            Schema schema = Schema.builder()
                    .namespace("telemetry")
                    .name("InferenceEvent")
                    .version("1.0.0")
                    .format(SchemaFormat.JSON_SCHEMA)
                    .content("{\"type\": \"object\", \"properties\": {\"model\": {\"type\": \"string\"}}}")
                    .build();

            // Register asynchronously with CompletableFuture
            client.registerSchema(schema)
                    .thenAccept(response -> {
                        System.out.println("Schema ID: " + response.getSchemaId());
                        System.out.println("Version: " + response.getVersion());
                    })
                    .join(); // Wait for completion

        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
```

## Usage Examples

### 1. Register a Schema

```java
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
        .build())
    .build();

client.registerSchema(schema)
    .thenAccept(response -> {
        System.out.println("Registered: " + response.getSchemaId());
    })
    .exceptionally(error -> {
        System.err.println("Error: " + error.getMessage());
        return null;
    });
```

### 2. Retrieve a Schema

```java
// By ID
client.getSchema("550e8400-e29b-41d4-a716-446655440000")
    .thenAccept(schema -> {
        System.out.println("Name: " + schema.getName());
        System.out.println("Version: " + schema.getVersion());
    });

// By namespace, name, and version
client.getSchemaByVersion("telemetry", "InferenceEvent", "1.0.0")
    .thenAccept(schema -> {
        System.out.println("Content: " + schema.getContent());
    });
```

### 3. Validate Data

```java
String schemaId = "550e8400-e29b-41d4-a716-446655440000";
String data = "{\"model\": \"gpt-4\"}";

client.validateData(schemaId, data)
    .thenAccept(result -> {
        if (result.isValid()) {
            System.out.println("Valid!");
        } else {
            System.out.println("Invalid. Errors:");
            result.getErrors().forEach(System.out::println);
        }
    });
```

### 4. Check Compatibility

```java
String schemaId = "550e8400-e29b-41d4-a716-446655440000";
String newSchema = "{\"type\": \"object\", \"properties\": {...}}";

client.checkCompatibility(schemaId, newSchema, CompatibilityMode.BACKWARD)
    .thenAccept(result -> {
        if (result.isCompatible()) {
            System.out.println("Compatible!");
        } else {
            System.out.println("Incompatible:");
            result.getIncompatibilities().forEach(System.out::println);
        }
    });
```

### 5. Search Schemas

```java
client.searchSchemas("inference", 10, 0)
    .thenAccept(results -> {
        results.forEach(result -> {
            System.out.println(result.getNamespace() + "." + result.getName());
        });
    });
```

### 6. Chain Operations

```java
client.registerSchema(schema)
    .thenCompose(response -> client.getSchema(response.getSchemaId()))
    .thenCompose(schema -> client.validateData(schema.getSchemaId(), data))
    .thenAccept(result -> {
        System.out.println("Valid: " + result.isValid());
    })
    .exceptionally(error -> {
        System.err.println("Error: " + error.getMessage());
        return null;
    });
```

### 7. Parallel Operations

```java
CompletableFuture<GetSchemaResponse> f1 = client.getSchema("id-1");
CompletableFuture<GetSchemaResponse> f2 = client.getSchema("id-2");
CompletableFuture<GetSchemaResponse> f3 = client.getSchema("id-3");

CompletableFuture.allOf(f1, f2, f3)
    .thenRun(() -> {
        try {
            System.out.println("Schema 1: " + f1.get().getName());
            System.out.println("Schema 2: " + f2.get().getName());
            System.out.println("Schema 3: " + f3.get().getName());
        } catch (Exception e) {
            e.printStackTrace();
        }
    });
```

## Configuration Options

### Client Builder

```java
SchemaRegistryClient client = SchemaRegistryClient.builder()
    .baseUrl("http://localhost:8080")           // Required: Registry base URL
    .apiKey("your-api-key")                     // Optional: API key for auth
    .timeout(Duration.ofSeconds(30))            // Optional: Request timeout (default: 30s)
    .maxRetries(3)                              // Optional: Max retry attempts (default: 3)
    .cacheTTL(Duration.ofMinutes(5))            // Optional: Cache TTL (default: 5 min)
    .cacheMaxSize(1000)                         // Optional: Max cache entries (default: 1000)
    .build();
```

## Exception Handling

The SDK provides specific exception types:

```java
try {
    client.getSchema("invalid-id").join();
} catch (CompletionException e) {
    Throwable cause = e.getCause();

    if (cause instanceof SchemaNotFoundException) {
        System.err.println("Schema not found (404)");
    } else if (cause instanceof AuthenticationException) {
        System.err.println("Authentication failed (401)");
    } else if (cause instanceof AuthorizationException) {
        System.err.println("Insufficient permissions (403)");
    } else if (cause instanceof IncompatibleSchemaException) {
        IncompatibleSchemaException ex = (IncompatibleSchemaException) cause;
        System.err.println("Incompatible: " + ex.getIncompatibilities());
    } else if (cause instanceof RateLimitException) {
        RateLimitException ex = (RateLimitException) cause;
        System.err.println("Rate limited. Retry after: " + ex.getRetryAfter());
    } else if (cause instanceof ServerException) {
        System.err.println("Server error (5xx)");
    } else if (cause instanceof SchemaRegistryException) {
        SchemaRegistryException ex = (SchemaRegistryException) cause;
        System.err.println("Error: " + ex.getMessage() + " (status: " + ex.getStatusCode() + ")");
    }
}
```

## Error Types

| Exception | Status Code | Description |
|-----------|-------------|-------------|
| `SchemaNotFoundException` | 404 | Schema not found |
| `AuthenticationException` | 401 | Authentication failed |
| `AuthorizationException` | 403 | Insufficient permissions |
| `IncompatibleSchemaException` | 409 | Schema compatibility check failed |
| `RateLimitException` | 429 | Rate limit exceeded |
| `ServerException` | 5xx | Server error |
| `SchemaRegistryException` | Various | Base exception for all errors |

## Best Practices

### 1. Use try-with-resources

Always close the client to release resources:

```java
try (SchemaRegistryClient client = SchemaRegistryClient.builder()
        .baseUrl("http://localhost:8080")
        .build()) {
    // Use client
}
```

### 2. Handle async exceptions

```java
client.getSchema(id)
    .thenAccept(schema -> { /* success */ })
    .exceptionally(error -> {
        logger.error("Failed to get schema", error);
        return null;
    });
```

### 3. Use caching effectively

```java
// Cache is enabled by default
client.getSchema(id); // Cache miss - fetches from server
client.getSchema(id); // Cache hit - returns immediately

// Disable cache for specific call
client.getSchema(id, false); // Always fetch from server

// Clear cache when needed
client.clearCache();
```

### 4. Implement retry logic

The client automatically retries failed requests with exponential backoff:
- Retries on server errors (5xx) and network failures
- Exponential backoff: 1s, 2s, 4s, 8s (max 10s)
- Configurable max retries (default: 3)

### 5. Use Builder pattern

```java
Schema schema = Schema.builder()
    .namespace("telemetry")
    .name("InferenceEvent")
    .version("1.0.0")
    .format(SchemaFormat.JSON_SCHEMA)
    .content("{...}")
    .metadata(SchemaMetadata.builder()
        .description("...")
        .addTag("ml")
        .owner("ml-team")
        .build())
    .build();
```

## API Reference

See the JavaDoc documentation for complete API reference:

```bash
mvn javadoc:javadoc
```

The generated documentation will be available in `target/site/apidocs/`.

## Building from Source

```bash
# Clone repository
git clone https://github.com/llm-schema-registry/llm-schema-registry.git
cd llm-schema-registry/sdks/java

# Build
mvn clean install

# Run tests
mvn test

# Generate JavaDoc
mvn javadoc:javadoc
```

## License

Apache License 2.0 - See [LICENSE](../../LICENSE) for details.

## Support

- GitHub Issues: [Report an issue](https://github.com/llm-schema-registry/llm-schema-registry/issues)
- Documentation: [Full documentation](../../README.md)
