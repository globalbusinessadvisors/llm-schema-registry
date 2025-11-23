# @llm-dev-ops/llm-schema-registry-integrations

LLM framework integrations for Schema Registry (LangChain, LlamaIndex, vLLM)

## Installation

```bash
npm install @llm-dev-ops/llm-schema-registry-integrations
```

## Usage

### LangChain Integration

```typescript
import { SchemaRegistryClient } from '@llm-dev-ops/llm-schema-registry-sdk';
import { createLangChainValidator } from '@llm-dev-ops/llm-schema-registry-integrations';

const client = new SchemaRegistryClient({
  baseURL: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

const validator = createLangChainValidator(
  client,
  'myapp',
  'user-schema',
  '1.0.0'
);

// Validate chain output
const result = await validator.validateChainOutput({
  name: 'John Doe',
  email: 'john@example.com'
});

if (result.valid) {
  console.log('Output is valid!');
} else {
  console.error('Validation errors:', result.errors);
}
```

### LlamaIndex Integration

```typescript
import { SchemaRegistryClient } from '@llm-dev-ops/llm-schema-registry-sdk';
import { createLlamaIndexValidator } from '@llm-dev-ops/llm-schema-registry-integrations';

const client = new SchemaRegistryClient({
  baseURL: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

const validator = createLlamaIndexValidator(
  client,
  'myapp',
  'document-schema',
  '1.0.0'
);

// Validate query response
const result = await validator.validateQueryResponse({
  documents: [{ id: '1', content: 'Hello world' }]
});

// Validate index data
const indexResult = await validator.validateIndexData({
  id: '1',
  metadata: { source: 'web' }
});
```

### vLLM Integration

```typescript
import { SchemaRegistryClient } from '@llm-dev-ops/llm-schema-registry-sdk';
import { createVLLMValidator } from '@llm-dev-ops/llm-schema-registry-integrations';

const client = new SchemaRegistryClient({
  baseURL: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

const validator = createVLLMValidator(
  client,
  'myapp',
  'generation-schema',
  '1.0.0'
);

// Validate single model output
const result = await validator.validateModelOutput({
  text: 'Generated response',
  tokens: 42
});

// Validate batch outputs
const batchResults = await validator.validateBatchOutput([
  { text: 'Response 1', tokens: 10 },
  { text: 'Response 2', tokens: 20 }
]);
```

## Features

- Schema validation for LangChain chain outputs
- Schema validation for LlamaIndex query responses and index data
- Schema validation for vLLM model outputs (single and batch)
- Full TypeScript support with type definitions
- Seamless integration with Schema Registry SDK

## Documentation

For more information, visit: https://github.com/globalbusinessadvisors/llm-schema-registry

## License

Apache-2.0
