import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// ============================================================================
// Soak Test - Long-duration test to detect memory leaks and degradation
// Duration: 2 hours at moderate load
// ============================================================================

const errorRate = new Rate('errors');
const latency = new Trend('latency');
const memoryLeakIndicator = Trend('memory_leak_indicator');
const requestCount = new Counter('total_requests');

export const options = {
  stages: [
    { duration: '5m', target: 200 },      // Ramp up
    { duration: '110m', target: 200 },    // Soak for 110 minutes (nearly 2 hours)
    { duration: '5m', target: 0 },        // Ramp down
  ],

  thresholds: {
    // Performance should not degrade over time
    'http_req_duration{scenario:read}': ['p(95)<10'],
    'http_req_duration{scenario:write}': ['p(95)<100'],

    // Error rate should remain low
    'errors': ['rate<0.01'],

    // No memory leaks (latency should not increase over time)
    'memory_leak_indicator': ['p(95)<15'],  // p95 latency should stay under 15ms
  },

  ext: {
    loadimpact: {
      projectID: 3639058,
      name: 'Schema Registry - Soak Test (2 hours)'
    }
  }
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

let schemaIds = [];
let startTime = Date.now();

export function setup() {
  console.log('Setting up soak test (2 hour duration)...');

  // Create initial schemas
  for (let i = 0; i < 100; i++) {
    const schema = {
      namespace: 'com.soak.test',
      name: `schema_${i}`,
      version_major: 1,
      version_minor: 0,
      version_patch: 0,
      format: 'JSON',
      content: JSON.stringify({
        '$schema': 'http://json-schema.org/draft-07/schema#',
        'type': 'object',
        'properties': {
          'id': { 'type': 'string' },
          'timestamp': { 'type': 'number' },
          'data': { 'type': 'string' }
        }
      }),
      state: 'ACTIVE'
    };

    const res = http.post(
      `${BASE_URL}/api/v1/schemas`,
      JSON.stringify(schema),
      { headers: { 'Content-Type': 'application/json', 'X-API-Key': API_KEY } }
    );

    if (res.status === 201 && res.json('id')) {
      schemaIds.push(res.json('id'));
    }
  }

  console.log(`Created ${schemaIds.length} schemas for soak test`);
  return { schemaIds, startTime: Date.now() };
}

export default function (data) {
  if (data && data.schemaIds) {
    schemaIds = data.schemaIds;
    startTime = data.startTime;
  }

  requestCount.add(1);

  // Mix of operations
  const rand = Math.random();

  if (rand < 0.70 && schemaIds.length > 0) {
    // 70% - Read existing schema
    const schemaId = schemaIds[Math.floor(Math.random() * schemaIds.length)];
    const reqStart = Date.now();

    const res = http.get(
      `${BASE_URL}/api/v1/schemas/${schemaId}`,
      {
        headers: { 'X-API-Key': API_KEY },
        tags: { scenario: 'read' },
      }
    );

    const duration = Date.now() - reqStart;
    latency.add(duration);

    // Track latency over time to detect degradation
    const testDuration = (Date.now() - startTime) / 1000 / 60; // minutes
    memoryLeakIndicator.add(duration);

    const success = check(res, {
      'read: status is 200': (r) => r.status === 200,
      'read: latency stable': () => duration < 20, // Should not degrade
    });

    if (!success) {
      errorRate.add(1);
    }
  } else if (rand < 0.85) {
    // 15% - Validate data
    if (schemaIds.length > 0) {
      const schemaId = schemaIds[Math.floor(Math.random() * schemaIds.length)];
      const reqStart = Date.now();

      const res = http.post(
        `${BASE_URL}/api/v1/validate`,
        JSON.stringify({
          schema_id: schemaId,
          data: {
            id: 'test-id',
            timestamp: Date.now(),
            data: 'test-data'
          }
        }),
        {
          headers: { 'Content-Type': 'application/json', 'X-API-Key': API_KEY },
          tags: { scenario: 'read' },
        }
      );

      const duration = Date.now() - reqStart;
      latency.add(duration);
      memoryLeakIndicator.add(duration);

      const success = check(res, {
        'validate: status is 200': (r) => r.status === 200,
      });

      if (!success) {
        errorRate.add(1);
      }
    }
  } else {
    // 15% - Create new schema
    const schema = {
      namespace: 'com.soak.test',
      name: `dynamic_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      version_major: 1,
      version_minor: 0,
      version_patch: 0,
      format: 'JSON',
      content: JSON.stringify({
        '$schema': 'http://json-schema.org/draft-07/schema#',
        'type': 'object',
        'properties': {
          'timestamp': { 'type': 'number' },
          'iteration': { 'type': 'number' }
        }
      }),
      state: 'ACTIVE'
    };

    const reqStart = Date.now();

    const res = http.post(
      `${BASE_URL}/api/v1/schemas`,
      JSON.stringify(schema),
      {
        headers: { 'Content-Type': 'application/json', 'X-API-Key': API_KEY },
        tags: { scenario: 'write' },
      }
    );

    const duration = Date.now() - reqStart;
    latency.add(duration);
    memoryLeakIndicator.add(duration);

    const success = check(res, {
      'write: status is 201': (r) => r.status === 201,
    });

    if (success && res.json('id')) {
      schemaIds.push(res.json('id'));

      // Prevent unbounded growth
      if (schemaIds.length > 10000) {
        schemaIds = schemaIds.slice(-5000);
      }
    } else {
      errorRate.add(1);
    }
  }

  // Realistic think time
  sleep(0.1);
}

export function teardown(data) {
  console.log('Soak test completed (2 hours)');
  const duration = (Date.now() - data.startTime) / 1000 / 60;
  console.log(`Actual duration: ${duration.toFixed(2)} minutes`);
  console.log(`Final schema count: ${data.schemaIds.length}`);
}
