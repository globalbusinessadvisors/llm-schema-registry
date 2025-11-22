// K6 Spike Test: Sudden traffic surge
// Tests how the system handles sudden load spikes

import http from 'k6/http';
import { check } from 'k6';

export const options = {
  stages: [
    // Normal load
    { duration: '2m', target: 100 },

    // Sudden spike
    { duration: '10s', target: 5000 }, // Spike to 5000 users in 10 seconds!

    // Stay at peak
    { duration: '3m', target: 5000 },

    // Drop back to normal
    { duration: '10s', target: 100 },

    // Recover
    { duration: '2m', target: 100 },

    // Ramp down
    { duration: '30s', target: 0 },
  ],

  thresholds: {
    'http_req_duration': ['p(99)<1000'], // 99% under 1s (degraded performance OK)
    'http_req_failed': ['rate<0.10'], // Allow up to 10% errors during spike
  },
};

const BASE_URL = __ENV.API_URL || 'http://localhost:8080';

export default function() {
  const response = http.get(`${BASE_URL}/health`);

  check(response, {
    'status is 200 or 503': (r) => r.status === 200 || r.status === 503,
  });
}
