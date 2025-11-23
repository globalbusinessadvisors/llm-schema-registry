import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';
export interface VLLMSchemaOptions {
    client: SchemaRegistryClient;
    namespace: string;
    schemaName: string;
    schemaVersion: string;
}
export declare class VLLMSchemaValidator {
    private client;
    private namespace;
    private schemaName;
    private schemaVersion;
    constructor(options: VLLMSchemaOptions);
    validate(data: unknown): Promise<boolean>;
    getSchema(): Promise<Schema>;
    validateModelOutput(output: unknown): Promise<{
        valid: boolean;
        errors?: string[];
    }>;
    validateBatchOutput(outputs: unknown[]): Promise<Array<{
        valid: boolean;
        errors?: string[];
    }>>;
}
export declare function createVLLMValidator(client: SchemaRegistryClient, namespace: string, schemaName: string, schemaVersion: string): VLLMSchemaValidator;
//# sourceMappingURL=vllm.d.ts.map