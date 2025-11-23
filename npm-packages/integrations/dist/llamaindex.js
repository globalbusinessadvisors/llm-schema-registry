"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.LlamaIndexSchemaValidator = void 0;
exports.createLlamaIndexValidator = createLlamaIndexValidator;
class LlamaIndexSchemaValidator {
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
    async validateQueryResponse(response) {
        const result = await this.client.validate(this.namespace, this.schemaName, this.schemaVersion, response);
        return {
            valid: result.is_valid,
            errors: result.errors
        };
    }
    async validateIndexData(data) {
        const result = await this.client.validate(this.namespace, this.schemaName, this.schemaVersion, data);
        return {
            valid: result.is_valid,
            errors: result.errors
        };
    }
}
exports.LlamaIndexSchemaValidator = LlamaIndexSchemaValidator;
function createLlamaIndexValidator(client, namespace, schemaName, schemaVersion) {
    return new LlamaIndexSchemaValidator({
        client,
        namespace,
        schemaName,
        schemaVersion
    });
}
//# sourceMappingURL=llamaindex.js.map