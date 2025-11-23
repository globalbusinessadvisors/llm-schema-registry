import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';

export interface LlamaIndexSchemaOptions {
  client: SchemaRegistryClient;
  namespace: string;
  schemaName: string;
  schemaVersion: string;
}

export class LlamaIndexSchemaValidator {
  private client: SchemaRegistryClient;
  private namespace: string;
  private schemaName: string;
  private schemaVersion: string;

  constructor(options: LlamaIndexSchemaOptions) {
    this.client = options.client;
    this.namespace = options.namespace;
    this.schemaName = options.schemaName;
    this.schemaVersion = options.schemaVersion;
  }

  async validate(data: unknown): Promise<boolean> {
    try {
      const result = await this.client.validate(
        this.namespace,
        this.schemaName,
        this.schemaVersion,
        data
      );
      return result.is_valid;
    } catch (error) {
      throw new Error(`Schema validation failed: ${error}`);
    }
  }

  async getSchema(): Promise<Schema> {
    const response = await this.client.getSchema(
      this.namespace,
      this.schemaName,
      this.schemaVersion
    );
    return {
      namespace: response.namespace,
      name: response.name,
      version: response.version,
      format: response.format,
      content: response.content,
      metadata: response.metadata
    };
  }

  async validateQueryResponse(response: unknown): Promise<{ valid: boolean; errors?: string[] }> {
    const result = await this.client.validate(
      this.namespace,
      this.schemaName,
      this.schemaVersion,
      response
    );

    return {
      valid: result.is_valid,
      errors: result.errors
    };
  }

  async validateIndexData(data: unknown): Promise<{ valid: boolean; errors?: string[] }> {
    const result = await this.client.validate(
      this.namespace,
      this.schemaName,
      this.schemaVersion,
      data
    );

    return {
      valid: result.is_valid,
      errors: result.errors
    };
  }
}

export function createLlamaIndexValidator(
  client: SchemaRegistryClient,
  namespace: string,
  schemaName: string,
  schemaVersion: string
): LlamaIndexSchemaValidator {
  return new LlamaIndexSchemaValidator({
    client,
    namespace,
    schemaName,
    schemaVersion
  });
}
