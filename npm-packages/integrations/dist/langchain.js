"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.LangChainSchemaValidator = void 0;
exports.createLangChainValidator = createLangChainValidator;
class LangChainSchemaValidator {
    constructor(options) {
        this.client = options.client;
        this.namespace = options.namespace;
        this.schemaName = options.schemaName;
        this.schemaVersion = options.schemaVersion;
    }
    async validate(data) {
        try {
            const result = await this.client.validate(this.namespace, this.schemaName, this.schemaVersion, data);
            return result.is_valid;
        }
        catch (error) {
            throw new Error(`Schema validation failed: ${error}`);
        }
    }
    async getSchema() {
        const response = await this.client.getSchema(this.namespace, this.schemaName, this.schemaVersion);
        return {
            namespace: response.namespace,
            name: response.name,
            version: response.version,
            format: response.format,
            content: response.content,
            metadata: response.metadata
        };
    }
    async validateChainOutput(output) {
        const result = await this.client.validate(this.namespace, this.schemaName, this.schemaVersion, output);
        return {
            valid: result.is_valid,
            errors: result.errors
        };
    }
}
exports.LangChainSchemaValidator = LangChainSchemaValidator;
function createLangChainValidator(client, namespace, schemaName, schemaVersion) {
    return new LangChainSchemaValidator({
        client,
        namespace,
        schemaName,
        schemaVersion
    });
}
//# sourceMappingURL=langchain.js.map