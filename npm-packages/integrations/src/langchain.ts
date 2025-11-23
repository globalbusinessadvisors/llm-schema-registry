import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';

export interface LangChainSchemaOptions {
  client: SchemaRegistryClient;
  namespace: string;
  schemaName: string;
  schemaVersion: string;
}

export class LangChainSchemaValidator {
  private client: SchemaRegistryClient;
  private namespace: string;
  private schemaName: string;
  private schemaVersion: string;

  constructor(options: LangChainSchemaOptions) {
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

  async validateChainOutput(output: unknown): Promise<{ valid: boolean; errors?: string[] }> {
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
}

export function createLangChainValidator(
  client: SchemaRegistryClient,
  namespace: string,
  schemaName: string,
  schemaVersion: string
): LangChainSchemaValidator {
  return new LangChainSchemaValidator({
    client,
    namespace,
    schemaName,
    schemaVersion
  });
}
