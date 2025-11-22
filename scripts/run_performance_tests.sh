#!/bin/bash

# ============================================================================
# Performance Testing and Validation Script
# ============================================================================
# This script runs comprehensive performance tests and generates reports
# Usage: ./scripts/run_performance_tests.sh [--full|--quick|--benchmarks-only]
# ============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_ROOT/performance-results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_DIR="$RESULTS_DIR/$TIMESTAMP"

# Test mode
TEST_MODE="${1:-full}"

# ============================================================================
# Helper Functions
# ============================================================================

log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $*"
}

success() {
    echo -e "${GREEN}[✓]${NC} $*"
}

warning() {
    echo -e "${YELLOW}[!]${NC} $*"
}

error() {
    echo -e "${RED}[✗]${NC} $*"
}

# ============================================================================
# Setup
# ============================================================================

setup() {
    log "Setting up performance testing environment..."

    # Create results directory
    mkdir -p "$REPORT_DIR"
    cd "$PROJECT_ROOT"

    # Check prerequisites
    if ! command -v cargo &> /dev/null; then
        error "cargo not found. Please install Rust."
        exit 1
    fi

    if ! command -v k6 &> /dev/null; then
        warning "k6 not found. Load tests will be skipped."
        warning "Install k6: https://k6.io/docs/getting-started/installation/"
    fi

    success "Setup complete"
}

# ============================================================================
# Benchmark Tests (Criterion)
# ============================================================================

run_benchmarks() {
    log "Running Criterion benchmarks..."

    # Build in release mode
    log "Building in release mode..."
    cargo build --release

    # Run all benchmarks
    log "Executing benchmarks..."
    cargo bench --all -- --verbose \
        | tee "$REPORT_DIR/criterion-benchmarks.txt"

    # Copy Criterion HTML reports
    if [ -d "target/criterion" ]; then
        cp -r target/criterion "$REPORT_DIR/criterion-html"
        success "Criterion HTML reports saved to $REPORT_DIR/criterion-html"
    fi

    success "Benchmarks complete"
}

# ============================================================================
# Database Performance Tests
# ============================================================================

run_database_tests() {
    log "Running database performance tests..."

    # Start PostgreSQL (if not already running)
    if ! pg_isready -h localhost -p 5432 &> /dev/null; then
        warning "PostgreSQL not running. Starting with Docker..."
        docker run -d \
            --name schema-registry-postgres-test \
            -e POSTGRES_PASSWORD=postgres \
            -e POSTGRES_DB=schema_registry_test \
            -p 5432:5432 \
            postgres:16
        sleep 5
    fi

    # Run migrations
    log "Running database migrations..."
    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/schema_registry_test"
    sqlx migrate run --database-url "$DATABASE_URL"

    # Run database benchmarks
    log "Testing query performance..."
    psql "$DATABASE_URL" <<EOF > "$REPORT_DIR/query-performance.txt"
-- Test query performance
EXPLAIN ANALYZE
SELECT * FROM schemas
WHERE namespace = 'com.example'
  AND name = 'test_schema'
  AND state = 'ACTIVE'
LIMIT 100;

-- Test index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;

-- Test table statistics
SELECT
    tablename,
    seq_scan,
    seq_tup_read,
    idx_scan,
    idx_tup_fetch,
    n_tup_ins,
    n_tup_upd,
    n_tup_del,
    n_live_tup,
    n_dead_tup
FROM pg_stat_user_tables
WHERE schemaname = 'public';
EOF

    success "Database tests complete"
}

# ============================================================================
# Load Tests (k6)
# ============================================================================

run_load_tests() {
    if ! command -v k6 &> /dev/null; then
        warning "Skipping load tests (k6 not installed)"
        return
    fi

    log "Running k6 load tests..."

    # Start server in background
    log "Starting schema registry server..."
    cargo run --release --bin schema-registry-server &
    SERVER_PID=$!
    sleep 10

    # Wait for server to be ready
    log "Waiting for server to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:8080/health &> /dev/null; then
            success "Server is ready"
            break
        fi
        sleep 1
    done

    # Run baseline load test
    log "Running baseline load test (1,000 req/sec)..."
    k6 run \
        --out json="$REPORT_DIR/baseline-load-results.json" \
        --summary-export="$REPORT_DIR/baseline-load-summary.json" \
        tests/load/baseline_load.js \
        || warning "Baseline load test failed"

    if [ "$TEST_MODE" == "full" ]; then
        # Run stress test
        log "Running stress test (10,000 req/sec)..."
        k6 run \
            --out json="$REPORT_DIR/stress-test-results.json" \
            --summary-export="$REPORT_DIR/stress-test-summary.json" \
            tests/load/stress_test.js \
            || warning "Stress test failed"
    fi

    # Stop server
    log "Stopping server..."
    kill $SERVER_PID || true
    wait $SERVER_PID 2>/dev/null || true

    success "Load tests complete"
}

# ============================================================================
# Memory Profiling
# ============================================================================

run_memory_profiling() {
    log "Running memory profiling..."

    if ! command -v heaptrack &> /dev/null; then
        warning "Skipping memory profiling (heaptrack not installed)"
        return
    fi

    # Build with debug symbols
    log "Building with debug symbols..."
    cargo build --release

    # Run with heaptrack
    log "Profiling memory usage..."
    heaptrack ./target/release/schema-registry-server &
    SERVER_PID=$!
    sleep 10

    # Run some load
    if command -v k6 &> /dev/null; then
        k6 run tests/load/baseline_load.js || true
    else
        sleep 60
    fi

    # Stop server
    kill $SERVER_PID || true
    wait $SERVER_PID 2>/dev/null || true

    # Move heaptrack results
    mv heaptrack.*.gz "$REPORT_DIR/" || true

    success "Memory profiling complete"
}

# ============================================================================
# CPU Profiling
# ============================================================================

run_cpu_profiling() {
    log "Running CPU profiling..."

    if ! command -v flamegraph &> /dev/null; then
        warning "Skipping CPU profiling (flamegraph not installed)"
        warning "Install: cargo install flamegraph"
        return
    fi

    log "Generating flamegraph..."
    sudo -n cargo flamegraph --bin schema-registry-server &
    SERVER_PID=$!
    sleep 10

    # Run some load
    if command -v k6 &> /dev/null; then
        k6 run tests/load/baseline_load.js || true
    else
        sleep 60
    fi

    # Stop server
    kill $SERVER_PID || true
    wait $SERVER_PID 2>/dev/null || true

    # Move flamegraph
    mv flamegraph.svg "$REPORT_DIR/" || true

    success "CPU profiling complete"
}

# ============================================================================
# Generate Report
# ============================================================================

generate_report() {
    log "Generating performance report..."

    cat > "$REPORT_DIR/PERFORMANCE_REPORT.md" <<EOF
# Performance Test Report

**Date:** $(date +'%Y-%m-%d %H:%M:%S')
**Commit:** $(git rev-parse --short HEAD)
**Mode:** $TEST_MODE

## Summary

This report contains results from comprehensive performance testing of the LLM Schema Registry.

## Test Results

### 1. Benchmark Tests (Criterion)

See \`criterion-benchmarks.txt\` for detailed benchmark results.
HTML reports available in \`criterion-html/\` directory.

**Key Metrics:**
- Schema registration: Target <100ms p95
- Schema retrieval: Target <10ms p95
- Validation: Target <50ms p95
- Compatibility check: Target <75ms p95

### 2. Database Performance

See \`query-performance.txt\` for query execution plans and index usage.

**Key Metrics:**
- All queries: Target <50ms
- Index hit rate: Target >95%
- Sequential scans: Should be minimal

### 3. Load Tests (k6)

See \`baseline-load-results.json\` and \`stress-test-results.json\` for detailed metrics.

**Baseline Load Test (1,000 req/sec):**
- Duration: 12 minutes
- Target throughput: 1,000 req/sec
- Target p95 latency: <10ms (reads), <100ms (writes)
- Target error rate: <1%

**Stress Test (10,000 req/sec):**
- Duration: 30 minutes
- Target throughput: 10,000 req/sec sustained, 15,000 req/sec peak
- Target p95 latency: <10ms (reads), <100ms (writes)
- Target error rate: <5% during peak

### 4. Memory Profiling

See \`heaptrack.*.gz\` files (open with \`heaptrack_gui\`).

**Key Metrics:**
- Total heap usage: Target <500MB per instance
- No memory leaks: Heap should stabilize
- Allocation rate: Target <100MB/sec

### 5. CPU Profiling

See \`flamegraph.svg\` (open in browser).

**Key Metrics:**
- Total CPU usage: Target <2 cores per instance
- No single function >10% CPU time
- Lock contention: <5% total CPU

## Performance SLOs

| Metric | Target | Status |
|--------|--------|--------|
| p50 retrieval latency | <5ms | ⏳ Pending |
| p95 retrieval latency | <10ms | ⏳ Pending |
| p99 retrieval latency | <25ms | ⏳ Pending |
| p95 registration latency | <100ms | ⏳ Pending |
| Sustained throughput | 10,000 req/sec | ⏳ Pending |
| Peak throughput | 15,000 req/sec | ⏳ Pending |
| Error rate | <1% | ⏳ Pending |
| Memory per instance | <500MB | ⏳ Pending |
| CPU per instance | <2 cores | ⏳ Pending |
| Cache hit rate | >95% | ⏳ Pending |

## Recommendations

### Performance Optimizations

1. **Database:**
   - Verify all indexes are being used (check query-performance.txt)
   - Add additional indexes if sequential scans detected
   - Tune connection pool settings

2. **Caching:**
   - Implement cache warming on startup
   - Monitor cache hit rates
   - Consider increasing cache TTL for hot data

3. **Application:**
   - Profile hot paths (see flamegraph.svg)
   - Reduce allocations in tight loops
   - Consider using \`Arc\` for shared data

### Next Steps

- [ ] Review benchmark results and identify slow operations
- [ ] Analyze flamegraph for CPU hotspots
- [ ] Check heaptrack for memory leaks
- [ ] Validate query execution plans
- [ ] Run soak test (2 hours) to detect degradation
- [ ] Test with production-like data volumes

## Files in This Report

- \`criterion-benchmarks.txt\` - Benchmark results
- \`criterion-html/\` - Interactive HTML reports
- \`query-performance.txt\` - Database query analysis
- \`baseline-load-results.json\` - Baseline load test results
- \`stress-test-results.json\` - Stress test results
- \`heaptrack.*.gz\` - Memory profiling data
- \`flamegraph.svg\` - CPU profiling visualization

## Environment

\`\`\`
$(uname -a)
\`\`\`

\`\`\`
$(cargo --version)
\`\`\`

\`\`\`
$(rustc --version)
\`\`\`

EOF

    success "Report generated: $REPORT_DIR/PERFORMANCE_REPORT.md"
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log "Starting performance testing (mode: $TEST_MODE)"

    setup

    case "$TEST_MODE" in
        quick)
            run_benchmarks
            ;;
        benchmarks-only)
            run_benchmarks
            ;;
        full)
            run_benchmarks
            run_database_tests
            run_load_tests
            run_memory_profiling
            run_cpu_profiling
            ;;
        *)
            run_benchmarks
            run_database_tests
            run_load_tests
            ;;
    esac

    generate_report

    success "Performance testing complete!"
    log "Results saved to: $REPORT_DIR"
    log ""
    log "View report: cat $REPORT_DIR/PERFORMANCE_REPORT.md"
    log "View flamegraph: open $REPORT_DIR/flamegraph.svg"
    log "View criterion results: open $REPORT_DIR/criterion-html/index.html"
}

# Run main function
main "$@"
