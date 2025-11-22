# Load Testing Suite

Comprehensive k6 load testing infrastructure for the LLM Schema Registry.

## Overview

This directory contains three load test scenarios designed to validate performance at different scales and durations:

1. **Baseline Load Test** - Warm-up and baseline performance (1,000 req/sec)
2. **Stress Test** - Target load validation (10,000 req/sec sustained, 15,000 req/sec spike)
3. **Soak Test** - Long-duration stability test (2 hours)

## Prerequisites

### Install k6

**macOS:**
```bash
brew install k6
```

**Linux:**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Windows:**
```powershell
winget install k6
```

**Docker:**
```bash
docker pull grafana/k6
```

### Start the Server

```bash
# From project root
cargo run --release --bin schema-registry-server
```

Wait for the server to be ready:
```bash
curl http://localhost:8080/health
```

## Test Scenarios

### 1. Baseline Load Test

**File:** `baseline_load.js`

**Purpose:** Establish baseline performance and warm up the cache.

**Configuration:**
- **Target:** 1,000 requests/second
- **Duration:** 12 minutes
- **Virtual Users:** 50-100
- **Traffic Mix:**
  - 60% Schema Retrieval (reads)
  - 20% Schema Validation
  - 15% Schema Registration (writes)
  - 5% Compatibility Checks

**Run:**
```bash
k6 run baseline_load.js
```

**With output:**
```bash
k6 run \
  --out json=baseline-results.json \
  --summary-export=baseline-summary.json \
  baseline_load.js
```

**Expected Results:**
- p95 read latency: <10ms
- p95 write latency: <100ms
- Error rate: <1%
- Throughput: >1,000 req/sec

---

### 2. Stress Test

**File:** `stress_test.js`

**Purpose:** Validate performance at target load (10K req/sec) and beyond (15K req/sec spike).

**Configuration:**
- **Target:** 10,000 requests/second sustained
- **Peak:** 15,000 requests/second (spike)
- **Duration:** 30 minutes
- **Virtual Users:** 100-1,500
- **Traffic Mix:**
  - 90% Reads
  - 10% Writes

**Stages:**
1. Warmup: 0→100 VUs (2m)
2. Ramp: 100→500 VUs (3m) - 5,000 req/sec
3. Sustain: 500 VUs (5m)
4. Target: 500→1,000 VUs (3m) - 10,000 req/sec
5. Sustain: 1,000 VUs (5m) - **Key validation period**
6. Spike: 1,000→1,500 VUs (1m) - 15,000 req/sec
7. Spike sustain: 1,500 VUs (2m)
8. Recovery: 1,500→500→100→0 VUs (6m)

**Run:**
```bash
k6 run stress_test.js
```

**With environment:**
```bash
API_URL=http://localhost:8080 \
API_KEY=your-api-key \
k6 run stress_test.js
```

**Expected Results:**
- p95 read latency: <10ms (sustained)
- p95 write latency: <100ms
- Error rate: <5% (during spike)
- Sustained throughput: >10,000 req/sec
- Peak throughput: >15,000 req/sec

---

### 3. Soak Test

**File:** `soak_test.js`

**Purpose:** Detect memory leaks and performance degradation over time.

**Configuration:**
- **Target:** 2,000 requests/second
- **Duration:** 2 hours (120 minutes)
- **Virtual Users:** 200
- **Traffic Mix:**
  - 70% Reads
  - 15% Validation
  - 15% Writes

**Run:**
```bash
k6 run soak_test.js
```

**Note:** This test takes 2 hours. Consider running overnight or in CI/CD.

**Expected Results:**
- Latency should NOT increase over time
- Memory usage should stabilize (no leaks)
- p95 latency: <15ms (should remain constant)
- Error rate: <1%

---

## Running Tests

### Quick Start

```bash
# 1. Start the server
cargo run --release --bin schema-registry-server &

# 2. Wait for server to be ready
sleep 10

# 3. Run baseline test
k6 run baseline_load.js

# 4. Run stress test
k6 run stress_test.js

# 5. Stop server
pkill schema-registry-server
```

### Using Docker

```bash
# Run k6 in Docker
docker run --rm -i \
  --network="host" \
  -v $PWD:/tests \
  grafana/k6 run /tests/baseline_load.js
```

### Automated Testing

Use the provided script for comprehensive testing:

```bash
# From project root
./scripts/run_performance_tests.sh --full
```

This runs:
- All benchmarks
- Database performance tests
- All load tests
- Memory profiling
- CPU profiling
- Generates comprehensive report

## Understanding Results

### Key Metrics

**Latency Metrics:**
- `p(50)`: Median latency - 50% of requests complete in this time
- `p(95)`: 95th percentile - 95% of requests complete in this time
- `p(99)`: 99th percentile - 99% of requests complete in this time

**Throughput Metrics:**
- `http_reqs`: Total requests made
- `rate`: Requests per second

**Error Metrics:**
- `http_req_failed`: Percentage of failed requests
- `errors`: Custom error rate metric

### Success Criteria

| Metric | Baseline | Stress | Soak |
|--------|----------|--------|------|
| **p95 Read Latency** | <10ms | <10ms | <15ms |
| **p95 Write Latency** | <100ms | <100ms | <100ms |
| **Error Rate** | <1% | <5% | <1% |
| **Throughput** | >1K req/sec | >10K req/sec | >2K req/sec |

### Interpreting Results

**Good Results:**
```
✓ http_req_duration{scenario:read}.....: avg=4.2ms  p(95)=8.5ms
✓ http_req_duration{scenario:write}....: avg=42ms   p(95)=87ms
✓ errors............................: 0.23%
✓ http_reqs.........................: rate=10234/s
```

**Problematic Results:**
```
✗ http_req_duration{scenario:read}.....: avg=45ms   p(95)=120ms  # Too slow
✗ errors............................: 5.4%                      # Too many errors
✗ http_reqs.........................: rate=3456/s               # Below target
```

## Troubleshooting

### Error: Connection Refused

**Problem:** k6 can't connect to the server.

**Solution:**
```bash
# Check if server is running
curl http://localhost:8080/health

# Check server logs
tail -f server.log

# Restart server
cargo run --release --bin schema-registry-server
```

### Error: Too Many Open Files

**Problem:** System file descriptor limit too low.

**Solution (Linux/macOS):**
```bash
ulimit -n 65536
```

**Solution (permanent):**
Edit `/etc/security/limits.conf`:
```
* soft nofile 65536
* hard nofile 65536
```

### Error Rate Too High

**Problem:** >5% errors during test.

**Troubleshooting:**
1. Check server logs for errors
2. Monitor CPU/memory usage: `htop`
3. Check database connections: `psql` and run `SELECT count(*) FROM pg_stat_activity;`
4. Verify Redis is running: `redis-cli ping`
5. Check rate limiting configuration

### Slow Response Times

**Problem:** Latency exceeds targets.

**Troubleshooting:**
1. Verify database indexes: Run `002_performance_indexes.sql`
2. Check cache hit rate: Should be >95%
3. Profile CPU: `cargo flamegraph`
4. Profile memory: `heaptrack`
5. Check connection pool saturation

## Custom Metrics

All tests export custom metrics:

```javascript
// Latency by operation
retrieval_latency       // Schema GET requests
registration_latency    // Schema POST requests
validation_latency      // Validation requests
compatibility_latency   // Compatibility check requests

// Counters
schemas_created         // Total schemas registered
schemas_retrieved       // Total schemas fetched
validations_conducted   // Total validations
compatibility_checks    // Total compatibility checks

// Gauges
active_connections      // Current active connections
```

View in results:
```bash
k6 run --summary-export=summary.json baseline_load.js
cat summary.json | jq '.metrics'
```

## Integration with Monitoring

### Export to Prometheus

Install the k6 Prometheus extension:

```bash
k6 run --out experimental-prometheus-rw baseline_load.js
```

### Export to Grafana Cloud

```bash
k6 run \
  --out cloud \
  baseline_load.js
```

### Export to InfluxDB

```bash
k6 run \
  --out influxdb=http://localhost:8086/k6 \
  baseline_load.js
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Load Tests

on:
  schedule:
    - cron: '0 2 * * 0'  # Weekly
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        ports:
          - 5432:5432
      redis:
        image: redis:7
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v3

      - name: Install k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6

      - name: Build
        run: cargo build --release

      - name: Start Server
        run: |
          ./target/release/schema-registry-server &
          sleep 10

      - name: Run Load Tests
        run: |
          k6 run tests/load/baseline_load.js
          k6 run tests/load/stress_test.js

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: |
            *.json
```

## Best Practices

1. **Always warm up** - Run baseline test before stress test
2. **Monitor resources** - Watch CPU, memory, disk I/O during tests
3. **Test incrementally** - Start small, increase gradually
4. **Use realistic data** - Match production schema sizes and complexity
5. **Test from multiple regions** - If deploying globally
6. **Run soak tests overnight** - 2 hours is significant time
7. **Compare baselines** - Track performance over time
8. **Document results** - Keep test results in git for trending

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `API_URL` | `http://localhost:8080` | Server URL |
| `API_KEY` | `test-api-key` | API key for authentication |

## Files

- `baseline_load.js` - Baseline test (450 lines)
- `stress_test.js` - Stress test (230 lines)
- `soak_test.js` - Soak test (220 lines)
- `README.md` - This file

## Resources

- [k6 Documentation](https://k6.io/docs/)
- [k6 Examples](https://k6.io/docs/examples/)
- [k6 Best Practices](https://k6.io/docs/misc/k6-best-practices/)
- [k6 Thresholds](https://k6.io/docs/using-k6/thresholds/)

## Support

For issues or questions:
1. Check server logs
2. Review PROFILING.md for performance analysis
3. Run automated script: `./scripts/run_performance_tests.sh`
4. Consult PERFORMANCE_VALIDATION_REPORT.md
