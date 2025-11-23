# LLM Schema Registry - NPM Packages

This directory contains all npm packages for the LLM Schema Registry project, published under the `@llm-dev-ops` organization.

## Available Packages

### 1. [@llm-dev-ops/llm-schema-registry-sdk](./typescript)

**TypeScript SDK for Schema Registry**

A complete TypeScript/JavaScript client library for interacting with the Schema Registry.

```bash
npm install @llm-dev-ops/llm-schema-registry-sdk
```

**Features:**
- Full CRUD operations for schemas
- Schema validation
- Compatibility checking
- LRU caching with configurable TTL
- Automatic retries with exponential backoff
- Type-safe API

### 2. [@llm-dev-ops/llm-schema-registry-cli](./cli)

**Command-line Interface**

A CLI tool for managing schemas from the command line.

```bash
npm install -g @llm-dev-ops/llm-schema-registry-cli
```

**Features:**
- Register, retrieve, update, and delete schemas
- Validate data against schemas
- Check schema compatibility
- List and search schemas
- Supports JSON Schema, Avro, and Protobuf

### 3. [@llm-dev-ops/llm-schema-registry-server](./server)

**HTTP/gRPC Server Binary**

The main Schema Registry server providing both HTTP REST and gRPC APIs.

```bash
npm install -g @llm-dev-ops/llm-schema-registry-server
```

**Features:**
- HTTP REST API (default port 8080)
- gRPC API (default port 50051)
- PostgreSQL/S3 storage backends
- Redis caching
- JWT authentication
- Prometheus metrics
- OpenTelemetry tracing

### 4. [@llm-dev-ops/llm-schema-api](./api)

**Core gRPC API Server**

High-performance gRPC API server for schema operations.

```bash
npm install -g @llm-dev-ops/llm-schema-api
```

**Features:**
- High-performance gRPC API
- Protocol Buffer definitions
- PostgreSQL storage with S3 backup
- Redis caching
- Streaming support
- Authentication and authorization

### 5. [@llm-dev-ops/llm-schema-registry-integrations](./integrations)

**LLM Framework Integrations**

Integrations for popular LLM frameworks (LangChain, LlamaIndex, vLLM).

```bash
npm install @llm-dev-ops/llm-schema-registry-integrations
```

**Features:**
- LangChain integration for chain output validation
- LlamaIndex integration for query/index data validation
- vLLM integration for model output validation
- TypeScript support with full type definitions

## Publishing

All packages are automatically published to npm when a new version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Or manually trigger the workflow via GitHub Actions.

## Package Structure

```
npm-packages/
├── cli/                 # CLI binary wrapper
├── server/             # Server binary wrapper
├── api/                # API binary wrapper
├── integrations/       # LLM framework integrations
└── README.md           # This file

sdks/
└── typescript/         # TypeScript SDK source
```

## Development

Each package can be tested with a dry-run before publishing:

```bash
cd npm-packages/cli
npm publish --dry-run

cd ../server
npm publish --dry-run

cd ../api
npm publish --dry-run

cd ../integrations
npm publish --dry-run

cd ../../sdks/typescript
npm publish --dry-run
```

## Requirements

- **Node.js**: >= 16.0.0
- **Platforms**: macOS (x64, arm64), Linux (x64, arm64), Windows (x64)

Binary packages (CLI, Server, API) will download pre-built binaries from GitHub releases or fall back to building from source using Cargo if available.

## License

All packages are licensed under Apache-2.0.
