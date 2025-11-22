// K6 Load Test: Basic load test for Schema Registry
// Target: 10,000 req/sec sustained

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const latency = new Trend('latency');
const schemaRegistrations = new Counter('schema_registrations');
const schemaRetrieval = new Counter('schema_retrievals');
const validationRequests = new Counter('validation_requests');

export const options = {
  stages: [
    // Ramp-up
    { duration: '2m', target: 100 },   // Ramp to 100 users over 2 min
    { duration: '2m', target: 500 },   // Ramp to 500 users over 2 min
    { duration: '2m', target: 1000 },  // Ramp to 1000 users over 2 min

    // Sustained load
    { duration: '10m', target: 1000 }, // Stay at 1000 users for 10 min

    // Peak load
    { duration: '2m', target: 2000 },  // Ramp to 2000 users
    { duration: '5m', target: 2000 },  // Stay at 2000 users for 5 min

    // Ramp-down
    { duration: '2m', target: 0 },     // Ramp down to 0
  ],

  thresholds: {
    // 95% of requests should be below 10ms for reads
    'http_req_duration{scenario:read}': ['p(95)<10'],

    // 95% of requests should be below 100ms for writes
    'http_req_duration{scenario:write}': ['p(95)<100'],

    // Error rate should be below 5%
    'errors': ['rate<0.05'],

    // Should achieve >10,000 req/sec
    'http_reqs': ['rate>10000'],

    // 99% of requests should succeed
    'http_req_failed': ['rate<0.01'],
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

// Pre-generated schema IDs for reads
const SCHEMA_IDS = [];
for (let i = 0; i < 100; i++) {
  SCHEMA_IDS.push(`schema-${i}`);
}

export function setup() {
  console.log('Setting up load test...');

  // Register 100 schemas for read tests
  for (let i = 0; i < 100; i++) {
    const schema = {
      subject: `test.schema.${i}`,
      schema: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          name: { type: 'string' },
          value: { type: 'number' },
        },
        required: ['id', 'name'],
      },
      schema_type: 'json',
    };

    const response = http.post(
      `${BASE_URL}/api/v1/schemas`,
      JSON.stringify(schema),
      {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': API_KEY,
        },
      }
    );

    if (response.status === 201) {
      const body = JSON.parse(response.body);
      SCHEMA_IDS[i] = body.id;
    }
  }

  console.log(`Setup complete. Registered ${SCHEMA_IDS.length} schemas.`);
  return { schemaIds: SCHEMA_IDS };
}

export default function(data) {
  // 80% reads, 15% writes, 5% validations (Pareto distribution)
  const rand = Math.random();

  if (rand < 0.80) {
    // READ: Get schema by ID
    const schemaId = data.schemaIds[Math.floor(Math.random() * data.schemaIds.length)];
    const startTime = new Date();

    const response = http.get(`${BASE_URL}/api/v1/schemas/${schemaId}`, {
      headers: { 'X-API-Key': API_KEY },
      tags: { scenario: 'read' },
    });

    const duration = new Date() - startTime;

    const success = check(response, {
      'status is 200': (r) => r.status === 200,
      'latency < 10ms': () => duration < 10,
      'has schema content': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.schema !== undefined;
        } catch {
          return false;
        }
      },
    });

    schemaRetrieval.add(1);
    errorRate.add(!success);
    latency.add(duration);

  } else if (rand < 0.95) {
    // WRITE: Register new schema
    const schemaNum = Math.floor(Math.random() * 1000000);
    const schema = {
      subject: `dynamic.schema.${schemaNum}`,
      schema: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          timestamp: { type: 'number' },
          data: { type: 'string' },
        },
        required: ['id'],
      },
      schema_type: 'json',
    };

    const startTime = new Date();

    const response = http.post(
      `${BASE_URL}/api/v1/schemas`,
      JSON.stringify(schema),
      {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': API_KEY,
        },
        tags: { scenario: 'write' },
      }
    );

    const duration = new Date() - startTime;

    const success = check(response, {
      'status is 201': (r) => r.status === 201 || r.status === 409, // 409 = duplicate
      'latency < 100ms': () => duration < 100,
      'has schema id': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.id !== undefined;
        } catch {
          return false;
        }
      },
    });

    schemaRegistrations.add(1);
    errorRate.add(!success);
    latency.add(duration);

  } else {
    // VALIDATE: Validate data against schema
    const schemaId = data.schemaIds[Math.floor(Math.random() * data.schemaIds.length)];
    const testData = {
      id: `test-${Math.floor(Math.random() * 10000)}`,
      name: 'Test Entity',
      value: Math.random() * 100,
    };

    const startTime = new Date();

    const response = http.post(
      `${BASE_URL}/api/v1/validate/${schemaId}`,
      JSON.stringify(testData),
      {
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': API_KEY,
        },
        tags: { scenario: 'validation' },
      }
    );

    const duration = new Date() - startTime;

    const success = check(response, {
      'status is 200': (r) => r.status === 200,
      'latency < 50ms': () => duration < 50,
      'has validation result': (r) => {
        try {
          const body = JSON.parse(r.body);
          return body.is_valid !== undefined;
        } catch {
          return false;
        }
      },
    });

    validationRequests.add(1);
    errorRate.add(!success);
    latency.add(duration);
  }

  // Think time between requests
  sleep(0.1);
}

export function teardown(data) {
  console.log('Load test complete.');
  console.log(`Total schemas registered: ${schemaRegistrations.value}`);
  console.log(`Total schemas retrieved: ${schemaRetrieval.value}`);
  console.log(`Total validations: ${validationRequests.value}`);
}
