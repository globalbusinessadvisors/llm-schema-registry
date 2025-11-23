# @llm-dev-ops/llm-schema-registry-cli

Command-line interface for the LLM Schema Registry.

## Installation

```bash
npm install -g @llm-dev-ops/llm-schema-registry-cli
```

## Usage

```bash
# Register a schema
llm-schema register --namespace myapp --name user-schema --version 1.0.0 --file schema.json

# Get a schema
llm-schema get --namespace myapp --name user-schema --version 1.0.0

# Validate data against a schema
llm-schema validate --namespace myapp --name user-schema --version 1.0.0 --data data.json

# List all schemas
llm-schema list --namespace myapp

# Check compatibility
llm-schema compat --namespace myapp --name user-schema --file new-schema.json
```

## Features

- Full CRUD operations for schemas
- Schema validation
- Compatibility checking
- Interactive and scriptable modes
- Supports JSON Schema, Avro, and Protobuf

## Documentation

For more information, visit: https://github.com/globalbusinessadvisors/llm-schema-registry

## License

Apache-2.0
