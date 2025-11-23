"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.VLLMSchemaValidator = void 0;
exports.createVLLMValidator = createVLLMValidator;
class VLLMSchemaValidator {
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
    async validateModelOutput(output) {
        const result = await this.client.validate(this.namespace, this.schemaName, this.schemaVersion, output);
        return {
            valid: result.is_valid,
            errors: result.errors
        };
    }
    async validateBatchOutput(outputs) {
        const results = await Promise.all(outputs.map(output => this.client.validate(this.namespace, this.schemaName, this.schemaVersion, output)));
        return results.map(result => ({
            valid: result.is_valid,
            errors: result.errors
        }));
    }
}
exports.VLLMSchemaValidator = VLLMSchemaValidator;
function createVLLMValidator(client, namespace, schemaName, schemaVersion) {
    return new VLLMSchemaValidator({
        client,
        namespace,
        schemaName,
        schemaVersion
    });
}
//# sourceMappingURL=vllm.js.map