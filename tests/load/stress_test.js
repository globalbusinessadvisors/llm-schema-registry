import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// ============================================================================
// Stress Test - Push system to limits and beyond
// Target: 10,000 req/sec sustained, spike to 15,000 req/sec
// ============================================================================

const errorRate = new Rate('errors');
const latency = new Trend('latency');

export const options = {
  stages: [
    // Warm-up
    { duration: '2m', target: 100 },      // 1,000 req/sec
    { duration: '3m', target: 100 },      // Sustain

    // Ramp to target load
    { duration: '3m', target: 500 },      // 5,000 req/sec
    { duration: '5m', target: 500 },      // Sustain

    // Target sustained load
    { duration: '3m', target: 1000 },     // 10,000 req/sec
    { duration: '5m', target: 1000 },     // Sustain for 5 minutes

    // Spike test
    { duration: '1m', target: 1500 },     // 15,000 req/sec
    { duration: '2m', target: 1500 },     // Sustain spike

    // Recovery
    { duration: '2m', target: 500 },      // Back to medium load
    { duration: '2m', target: 100 },      // Back to low load

    // Ramp down
    { duration: '2m', target: 0 },
  ],

  thresholds: {
    // Latency thresholds
    'http_req_duration{scenario:read}': [
      'p(50)<5',      // 50% under 5ms
      'p(95)<10',     // 95% under 10ms
      'p(99)<25',     // 99% under 25ms
    ],
    'http_req_duration{scenario:write}': [
      'p(95)<100',    // 95% under 100ms
      'p(99)<250',    // 99% under 250ms
    ],

    // Error rate - allow up to 5% during spike
    'errors': ['rate<0.05'],

    // Throughput - must sustain 10K req/sec
    'http_reqs': ['rate>10000'],

    // HTTP success rate
    'http_req_failed': ['rate<0.05'],
  },

  ext: {
    loadimpact: {
      projectID: 3639058,
      name: 'Schema Registry - Stress Test (10K req/sec)'
    }
  }
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

// Shared schema IDs (populated during setup)
let schemaIds = [];

export function setup() {
  console.log('Setting up stress test...');

  // Create 100 schemas for retrieval
  for (let i = 0; i < 100; i++) {
    const schema = {
      namespace: 'com.stress.test',
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
          'value': { 'type': 'number' }
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

  console.log(`Created ${schemaIds.length} schemas for stress test`);
  return { schemaIds };
}

export default function (data) {
  // Use data from setup
  if (data && data.schemaIds && data.schemaIds.length > 0) {
    schemaIds = data.schemaIds;
  }

  // 90% reads, 10% writes (read-heavy load)
  const rand = Math.random();

  if (rand < 0.90 && schemaIds.length > 0) {
    // Read operation
    const schemaId = schemaIds[Math.floor(Math.random() * schemaIds.length)];
    const startTime = Date.now();

    const res = http.get(
      `${BASE_URL}/api/v1/schemas/${schemaId}`,
      {
        headers: { 'X-API-Key': API_KEY },
        tags: { scenario: 'read' },
      }
    );

    latency.add(Date.now() - startTime);

    const success = check(res, {
      'status is 200': (r) => r.status === 200,
    });

    if (!success) {
      errorRate.add(1);
    }
  } else {
    // Write operation
    const schema = {
      namespace: 'com.stress.test',
      name: `dynamic_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      version_major: 1,
      version_minor: 0,
      version_patch: 0,
      format: 'JSON',
      content: JSON.stringify({
        '$schema': 'http://json-schema.org/draft-07/schema#',
        'type': 'object',
        'properties': {
          'timestamp': { 'type': 'number' }
        }
      }),
      state: 'ACTIVE'
    };

    const startTime = Date.now();

    const res = http.post(
      `${BASE_URL}/api/v1/schemas`,
      JSON.stringify(schema),
      {
        headers: { 'Content-Type': 'application/json', 'X-API-Key': API_KEY },
        tags: { scenario: 'write' },
      }
    );

    latency.add(Date.now() - startTime);

    const success = check(res, {
      'status is 201': (r) => r.status === 201,
    });

    if (success && res.json('id')) {
      schemaIds.push(res.json('id'));

      // Keep array size manageable
      if (schemaIds.length > 1000) {
        schemaIds = schemaIds.slice(-1000);
      }
    } else {
      errorRate.add(1);
    }
  }

  // Minimal think time for maximum throughput
  sleep(0.01); // 10ms
}

export function teardown(data) {
  console.log('Stress test completed');
  console.log(`Final schema count: ${data?.schemaIds?.length || 0}`);
}
