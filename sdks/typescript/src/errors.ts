/**
 * Error classes for the LLM Schema Registry TypeScript SDK
 */

/** Base error for all schema registry errors */
export class SchemaRegistryError extends Error {
  constructor(
    message: string,
    public readonly statusCode?: number
  ) {
    super(message);
    this.name = 'SchemaRegistryError';
    Object.setPrototypeOf(this, SchemaRegistryError.prototype);
  }
}

/** Error thrown when a schema is not found */
export class SchemaNotFoundError extends SchemaRegistryError {
  constructor(public readonly schemaId: string) {
    super(`Schema not found: ${schemaId}`, 404);
    this.name = 'SchemaNotFoundError';
    Object.setPrototypeOf(this, SchemaNotFoundError.prototype);
  }
}

/** Error thrown when schema validation fails */
export class SchemaValidationError extends SchemaRegistryError {
  constructor(public readonly errors: string[]) {
    const message = 'Schema validation failed:\n' + errors.map((e) => `  - ${e}`).join('\n');
    super(message, 400);
    this.name = 'SchemaValidationError';
    Object.setPrototypeOf(this, SchemaValidationError.prototype);
  }
}

/** Error thrown when schemas are incompatible */
export class IncompatibleSchemaError extends SchemaRegistryError {
  constructor(public readonly incompatibilities: string[]) {
    const message =
      'Schema compatibility check failed:\n' + incompatibilities.map((i) => `  - ${i}`).join('\n');
    super(message, 409);
    this.name = 'IncompatibleSchemaError';
    Object.setPrototypeOf(this, IncompatibleSchemaError.prototype);
  }
}

/** Error thrown when authentication fails */
export class AuthenticationError extends SchemaRegistryError {
  constructor(message: string = 'Authentication failed') {
    super(message, 401);
    this.name = 'AuthenticationError';
    Object.setPrototypeOf(this, AuthenticationError.prototype);
  }
}

/** Error thrown when authorization fails */
export class AuthorizationError extends SchemaRegistryError {
  constructor(message: string = 'Insufficient permissions') {
    super(message, 403);
    this.name = 'AuthorizationError';
    Object.setPrototypeOf(this, AuthorizationError.prototype);
  }
}

/** Error thrown when rate limit is exceeded */
export class RateLimitError extends SchemaRegistryError {
  constructor(public readonly retryAfter?: number) {
    const message = retryAfter
      ? `Rate limit exceeded. Retry after ${retryAfter} seconds`
      : 'Rate limit exceeded';
    super(message, 429);
    this.name = 'RateLimitError';
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/** Error thrown when server encounters an error */
export class ServerError extends SchemaRegistryError {
  constructor(message: string = 'Internal server error') {
    super(message, 500);
    this.name = 'ServerError';
    Object.setPrototypeOf(this, ServerError.prototype);
  }
}
