import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';
export interface LlamaIndexSchemaOptions {
    client: SchemaRegistryClient;
    namespace: string;
    schemaName: string;
    schemaVersion: string;
}
export declare class LlamaIndexSchemaValidator {
    private client;
    private namespace;
    private schemaName;
    private schemaVersion;
    constructor(options: LlamaIndexSchemaOptions);
    validate(data: unknown): Promise<boolean>;
    getSchema(): Promise<Schema>;
    validateQueryResponse(response: unknown): Promise<{
        valid: boolean;
        errors?: string[];
    }>;
    validateIndexData(data: unknown): Promise<{
        valid: boolean;
        errors?: string[];
    }>;
}
export declare function createLlamaIndexValidator(client: SchemaRegistryClient, namespace: string, schemaName: string, schemaVersion: string): LlamaIndexSchemaValidator;
//# sourceMappingURL=llamaindex.d.ts.map