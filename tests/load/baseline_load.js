import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';
import { randomString, randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

// ============================================================================
// Custom Metrics
// ============================================================================

const errorRate = new Rate('errors');
const retrievalLatency = new Trend('retrieval_latency');
const registrationLatency = new Trend('registration_latency');
const validationLatency = new Trend('validation_latency');
const compatibilityLatency = new Trend('compatibility_latency');

const schemasCreated = new Counter('schemas_created');
const schemasRetrieved = new Counter('schemas_retrieved');
const validationsConducted = new Counter('validations_conducted');
const compatibilityChecks = new Counter('compatibility_checks');

const activeConnections = new Gauge('active_connections');

// ============================================================================
// Test Configuration
// ============================================================================

export const options = {
  // Baseline load test: Ramp up to 1,000 req/sec for warm-up
  stages: [
    { duration: '2m', target: 50 },    // Ramp-up to 50 VUs
    { duration: '3m', target: 100 },   // Ramp to 100 VUs
    { duration: '5m', target: 100 },   // Stay at 100 VUs
    { duration: '2m', target: 0 },     // Ramp-down
  ],

  thresholds: {
    // HTTP metrics
    'http_req_duration': ['p(95)<100'],           // 95% of requests < 100ms
    'http_req_duration{scenario:read}': ['p(95)<10'],    // Reads < 10ms
    'http_req_duration{scenario:write}': ['p(95)<100'],  // Writes < 100ms

    // Custom metrics
    'retrieval_latency': ['p(95)<10'],             // Schema retrieval < 10ms
    'registration_latency': ['p(95)<100'],         // Schema registration < 100ms
    'validation_latency': ['p(95)<50'],            // Validation < 50ms
    'compatibility_latency': ['p(95)<75'],         // Compatibility check < 75ms

    // Error rate
    'errors': ['rate<0.01'],                       // Error rate < 1%
    'http_req_failed': ['rate<0.01'],              // HTTP failure rate < 1%

    // Throughput
    'http_reqs': ['rate>1000'],                    // > 1000 req/sec
  },

  // Environment
  ext: {
    loadimpact: {
      projectID: 3639058,
      name: 'Schema Registry - Baseline Load Test'
    }
  }
};

// ============================================================================
// Configuration
// ============================================================================

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

// Pre-generated test data
const NAMESPACES = ['com.example', 'com.test', 'org.acme', 'io.platform'];
const SCHEMA_NAMES = ['user', 'order', 'product', 'event', 'transaction', 'log'];
const FORMATS = ['JSON', 'AVRO', 'PROTOBUF'];

// ============================================================================
// Helper Functions
// ============================================================================

function generateJsonSchema(name, complexity = 10) {
  const properties = {};
  for (let i = 0; i < complexity; i++) {
    properties[`field_${i}`] = {
      type: randomIntBetween(0, 1) ? 'string' : 'number',
      description: `Field ${i} for ${name}`
    };
  }

  return {
    '$schema': 'http://json-schema.org/draft-07/schema#',
    'type': 'object',
    'title': name,
    'properties': properties,
    'required': Object.keys(properties).slice(0, Math.floor(complexity / 2))
  };
}

function generateAvroSchema(name, complexity = 10) {
  const fields = [];
  for (let i = 0; i < complexity; i++) {
    fields.push({
      name: `field_${i}`,
      type: randomIntBetween(0, 1) ? 'string' : 'int',
      doc: `Field ${i} for ${name}`
    });
  }

  return JSON.stringify({
    type: 'record',
    name: name.replace(/[^a-zA-Z0-9]/g, '_'),
    namespace: 'com.example',
    fields: fields
  });
}

function getRandomElement(array) {
  return array[randomIntBetween(0, array.length - 1)];
}

function buildHeaders() {
  return {
    'Content-Type': 'application/json',
    'X-API-Key': API_KEY,
  };
}

// Keep track of created schema IDs for retrieval
let createdSchemaIds = [];

// ============================================================================
// Test Scenarios
// ============================================================================

export default function () {
  activeConnections.add(1);

  // Pareto distribution: 80% reads, 20% writes
  const rand = Math.random();

  if (rand < 0.60) {
    // 60%: Schema retrieval (read-heavy workload)
    testSchemaRetrieval();
  } else if (rand < 0.80) {
    // 20%: Schema validation
    testSchemaValidation();
  } else if (rand < 0.95) {
    // 15%: Schema registration (write)
    testSchemaRegistration();
  } else {
    // 5%: Compatibility check
    testCompatibilityCheck();
  }

  activeConnections.add(-1);

  // Think time (realistic user behavior)
  sleep(randomIntBetween(1, 3) / 10); // 0.1-0.3 seconds
}

function testSchemaRetrieval() {
  // If we have created schemas, retrieve them; otherwise, create some first
  if (createdSchemaIds.length === 0) {
    testSchemaRegistration();
    return;
  }

  const schemaId = getRandomElement(createdSchemaIds);
  const startTime = Date.now();

  const res = http.get(
    `${BASE_URL}/api/v1/schemas/${schemaId}`,
    {
      headers: buildHeaders(),
      tags: { scenario: 'read', operation: 'retrieve' },
    }
  );

  const duration = Date.now() - startTime;
  retrievalLatency.add(duration);
  schemasRetrieved.add(1);

  const success = check(res, {
    'retrieval: status is 200': (r) => r.status === 200,
    'retrieval: has schema data': (r) => r.json('id') !== undefined,
    'retrieval: latency < 10ms': () => duration < 10,
  });

  if (!success) {
    errorRate.add(1);
  }
}

function testSchemaRegistration() {
  const namespace = getRandomElement(NAMESPACES);
  const name = getRandomElement(SCHEMA_NAMES) + '_' + randomString(8);
  const format = getRandomElement(FORMATS);

  let schemaContent;
  if (format === 'JSON') {
    schemaContent = generateJsonSchema(name, randomIntBetween(5, 20));
  } else if (format === 'AVRO') {
    schemaContent = generateAvroSchema(name, randomIntBetween(5, 20));
  } else {
    // PROTOBUF - simplified as JSON for now
    schemaContent = generateJsonSchema(name, randomIntBetween(5, 20));
  }

  const payload = JSON.stringify({
    namespace: namespace,
    name: name,
    version_major: 1,
    version_minor: 0,
    version_patch: 0,
    format: format,
    content: typeof schemaContent === 'string' ? schemaContent : JSON.stringify(schemaContent),
    state: 'ACTIVE',
    compatibility_mode: 'BACKWARD',
    description: `Test schema for ${name}`
  });

  const startTime = Date.now();

  const res = http.post(
    `${BASE_URL}/api/v1/schemas`,
    payload,
    {
      headers: buildHeaders(),
      tags: { scenario: 'write', operation: 'register' },
    }
  );

  const duration = Date.now() - startTime;
  registrationLatency.add(duration);

  const success = check(res, {
    'registration: status is 201': (r) => r.status === 201,
    'registration: returns schema ID': (r) => r.json('id') !== undefined,
    'registration: latency < 100ms': () => duration < 100,
  });

  if (success && res.json('id')) {
    createdSchemaIds.push(res.json('id'));
    schemasCreated.add(1);

    // Keep array size manageable
    if (createdSchemaIds.length > 1000) {
      createdSchemaIds = createdSchemaIds.slice(-1000);
    }
  } else {
    errorRate.add(1);
  }
}

function testSchemaValidation() {
  // Validate some data against a schema
  if (createdSchemaIds.length === 0) {
    testSchemaRegistration();
    return;
  }

  const schemaId = getRandomElement(createdSchemaIds);

  // Generate sample data
  const testData = {
    field_0: 'test_value',
    field_1: 42,
    field_2: 'another_value',
  };

  const payload = JSON.stringify({
    schema_id: schemaId,
    data: testData
  });

  const startTime = Date.now();

  const res = http.post(
    `${BASE_URL}/api/v1/validate`,
    payload,
    {
      headers: buildHeaders(),
      tags: { scenario: 'read', operation: 'validate' },
    }
  );

  const duration = Date.now() - startTime;
  validationLatency.add(duration);
  validationsConducted.add(1);

  const success = check(res, {
    'validation: status is 200': (r) => r.status === 200,
    'validation: has result': (r) => r.json('valid') !== undefined,
    'validation: latency < 50ms': () => duration < 50,
  });

  if (!success) {
    errorRate.add(1);
  }
}

function testCompatibilityCheck() {
  // Check compatibility between two schemas
  if (createdSchemaIds.length < 2) {
    testSchemaRegistration();
    return;
  }

  const schemaId1 = getRandomElement(createdSchemaIds);
  const schemaId2 = getRandomElement(createdSchemaIds);

  const payload = JSON.stringify({
    schema_id: schemaId1,
    compared_schema_id: schemaId2,
    mode: getRandomElement(['BACKWARD', 'FORWARD', 'FULL', 'NONE'])
  });

  const startTime = Date.now();

  const res = http.post(
    `${BASE_URL}/api/v1/compatibility/check`,
    payload,
    {
      headers: buildHeaders(),
      tags: { scenario: 'read', operation: 'compatibility' },
    }
  );

  const duration = Date.now() - startTime;
  compatibilityLatency.add(duration);
  compatibilityChecks.add(1);

  const success = check(res, {
    'compatibility: status is 200': (r) => r.status === 200,
    'compatibility: has result': (r) => r.json('is_compatible') !== undefined,
    'compatibility: latency < 75ms': () => duration < 75,
  });

  if (!success) {
    errorRate.add(1);
  }
}

// ============================================================================
// Setup and Teardown
// ============================================================================

export function setup() {
  console.log('Starting baseline load test...');
  console.log(`Target URL: ${BASE_URL}`);
  console.log('Creating initial test data...');

  // Create some initial schemas for retrieval tests
  for (let i = 0; i < 50; i++) {
    const namespace = getRandomElement(NAMESPACES);
    const name = `setup_schema_${i}`;
    const schema = generateJsonSchema(name, 10);

    const payload = JSON.stringify({
      namespace: namespace,
      name: name,
      version_major: 1,
      version_minor: 0,
      version_patch: 0,
      format: 'JSON',
      content: JSON.stringify(schema),
      state: 'ACTIVE',
      compatibility_mode: 'BACKWARD'
    });

    const res = http.post(
      `${BASE_URL}/api/v1/schemas`,
      payload,
      { headers: buildHeaders() }
    );

    if (res.status === 201 && res.json('id')) {
      createdSchemaIds.push(res.json('id'));
    }
  }

  console.log(`Created ${createdSchemaIds.length} initial schemas`);

  return { schemaIds: createdSchemaIds };
}

export function teardown(data) {
  console.log('Load test completed');
  console.log(`Total schemas created: ${data.schemaIds.length}`);
}
