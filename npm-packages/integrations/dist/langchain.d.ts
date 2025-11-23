import { SchemaRegistryClient, Schema } from '@llm-dev-ops/llm-schema-registry-sdk';
export interface LangChainSchemaOptions {
    client: SchemaRegistryClient;
    namespace: string;
    schemaName: string;
    schemaVersion: string;
}
export declare class LangChainSchemaValidator {
    private client;
    private namespace;
    private schemaName;
    private schemaVersion;
    constructor(options: LangChainSchemaOptions);
    validate(data: unknown): Promise<boolean>;
    getSchema(): Promise<Schema>;
    validateChainOutput(output: unknown): Promise<{
        valid: boolean;
        errors?: string[];
    }>;
}
export declare function createLangChainValidator(client: SchemaRegistryClient, namespace: string, schemaName: string, schemaVersion: string): LangChainSchemaValidator;
//# sourceMappingURL=langchain.d.ts.map