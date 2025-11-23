import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';

export interface VLLMSchemaOptions {
  client: SchemaRegistryClient;
  namespace: string;
  schemaName: string;
  schemaVersion: string;
}

export class VLLMSchemaValidator {
  private client: SchemaRegistryClient;
  private namespace: string;
  private schemaName: string;
  private schemaVersion: string;

  constructor(options: VLLMSchemaOptions) {
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

  async validateModelOutput(output: unknown): Promise<{ valid: boolean; errors?: string[] }> {
    const result = await this.client.validate(
      this.namespace,
      this.schemaName,
      this.schemaVersion,
      output
    );

    return {
      valid: result.is_valid,
      errors: result.errors
    };
  }

  async validateBatchOutput(outputs: unknown[]): Promise<Array<{ valid: boolean; errors?: string[] }>> {
    const results = await Promise.all(
      outputs.map(output =>
        this.client.validate(
          this.namespace,
          this.schemaName,
          this.schemaVersion,
          output
        )
      )
    );

    return results.map(result => ({
      valid: result.is_valid,
      errors: result.errors
    }));
  }
}

export function createVLLMValidator(
  client: SchemaRegistryClient,
  namespace: string,
  schemaName: string,
  schemaVersion: string
): VLLMSchemaValidator {
  return new VLLMSchemaValidator({
    client,
    namespace,
    schemaName,
    schemaVersion
  });
}
