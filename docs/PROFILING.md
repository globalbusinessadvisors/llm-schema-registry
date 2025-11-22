# Performance Profiling Guide

This guide covers memory and CPU profiling for the LLM Schema Registry.

## Table of Contents

- [Prerequisites](#prerequisites)
- [CPU Profiling](#cpu-profiling)
- [Memory Profiling](#memory-profiling)
- [Analyzing Results](#analyzing-results)
- [Continuous Profiling](#continuous-profiling)

## Prerequisites

Install profiling tools:

```bash
# Flamegraph tools
cargo install flamegraph

# Heaptrack (Linux)
sudo apt-get install heaptrack

# Valgrind (Linux)
sudo apt-get install valgrind

# perf (Linux)
sudo apt-get install linux-tools-common linux-tools-generic
```

## CPU Profiling

### Using Flamegraph

Flamegraph creates visual representations of CPU usage:

```bash
# Profile the server (requires root for perf)
sudo cargo flamegraph --bin schema-registry-server

# Profile with specific load test
sudo cargo flamegraph --bin schema-registry-server &
SERVER_PID=$!
sleep 10  # Wait for server to start
k6 run tests/load/baseline_load.js
kill $SERVER_PID
```

This generates `flamegraph.svg` showing CPU hotspots.

### Using perf

For more detailed CPU profiling:

```bash
# Build with debug symbols
cargo build --release

# Start server
./target/release/schema-registry-server &
SERVER_PID=$!

# Record performance data
sudo perf record -F 99 -p $SERVER_PID -g -- sleep 60

# Generate report
sudo perf report

# Generate flamegraph
sudo perf script | stackcollapse-perf.pl | flamegraph.pl > perf-flamegraph.svg
```

### Interpreting CPU Profiles

Key areas to investigate:

1. **Hot Functions**: Functions consuming >5% CPU
   - Look for unnecessary allocations
   - Check for inefficient algorithms
   - Identify blocking operations in async code

2. **Lock Contention**: High CPU in `parking_lot` or `tokio::sync`
   - Consider lock-free alternatives
   - Reduce critical section size
   - Use read-write locks appropriately

3. **Serialization**: Time in `serde_json`, `bincode`
   - Use zero-copy deserialization where possible
   - Consider binary formats for hot paths
   - Cache serialized results

## Memory Profiling

### Using Heaptrack

Heaptrack provides detailed memory allocation tracking:

```bash
# Profile server
heaptrack ./target/release/schema-registry-server

# Run load test while profiling
k6 run tests/load/stress_test.js

# Analyze results (generates heaptrack GUI)
heaptrack_gui heaptrack.schema-registry-server.*.gz
```

### Using Valgrind Massif

Massif tracks heap memory over time:

```bash
# Profile with massif
valgrind --tool=massif \
         --massif-out-file=massif.out \
         ./target/release/schema-registry-server

# Visualize results
ms_print massif.out > massif-report.txt

# Or use GUI
massif-visualizer massif.out
```

### Using DHAT (Dynamic Heap Analysis Tool)

For detailed heap profiling:

```bash
valgrind --tool=dhat \
         --dhat-out-file=dhat.out \
         ./target/release/schema-registry-server

# View results
firefox dhat.html  # Opens visualization
```

### Memory Leak Detection

Use Valgrind's memcheck:

```bash
valgrind --leak-check=full \
         --show-leak-kinds=all \
         --track-origins=yes \
         --verbose \
         --log-file=valgrind-out.txt \
         ./target/release/schema-registry-server
```

### Rust-Specific Memory Profiling

Add `jemalloc` for better memory profiling:

```toml
# Cargo.toml
[dependencies]
tikv-jemallocator = "0.5"

[profile.release]
debug = true  # Keep symbols for profiling
```

```rust
// main.rs
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

Then use `jemalloc` profiling:

```bash
MALLOC_CONF=prof:true,prof_prefix:jeprof.out ./target/release/schema-registry-server

# Generate heap profile
jeprof --show_bytes --pdf ./target/release/schema-registry-server jeprof.out.*.heap > heap-profile.pdf
```

### Interpreting Memory Profiles

Key metrics:

1. **Total Heap Usage**: Should stabilize under load
   - Target: <500MB per instance
   - Watch for linear growth (memory leak)

2. **Allocation Hot Spots**: Functions allocating most memory
   - Use `Arc` for shared data
   - Pool buffers for reuse
   - Avoid unnecessary clones

3. **Fragmentation**: Difference between allocated and used memory
   - Consider custom allocators
   - Use sized allocations
   - Pool frequently allocated objects

## Analyzing Results

### CPU Analysis Checklist

- [ ] No single function >10% CPU time
- [ ] Async functions don't block
- [ ] Lock contention <5% total CPU
- [ ] Serialization <15% total CPU
- [ ] Database queries <20% total CPU

### Memory Analysis Checklist

- [ ] Heap usage stable under sustained load
- [ ] No memory leaks (heap grows unbounded)
- [ ] Total memory <500MB per instance
- [ ] Allocation rate reasonable (<100MB/sec)
- [ ] Fragmentation <20%

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| CPU Usage | <2 cores | `top`, `htop` |
| Memory Usage | <500MB | `heaptrack`, `massif` |
| Allocations/sec | <1M | `heaptrack` |
| Hot Function Time | <10% each | `flamegraph` |
| GC Pressure | N/A (Rust) | - |

## Continuous Profiling

### Automated Profiling in CI

```yaml
# .github/workflows/profiling.yml
name: Performance Profiling

on:
  schedule:
    - cron: '0 2 * * 0'  # Weekly on Sunday at 2 AM
  workflow_dispatch:

jobs:
  profile:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install profiling tools
        run: |
          sudo apt-get update
          sudo apt-get install -y linux-tools-generic heaptrack

      - name: Build with profiling
        run: cargo build --release

      - name: CPU Profiling
        run: |
          sudo cargo flamegraph --bin schema-registry-server &
          sleep 10
          k6 run tests/load/baseline_load.js
          pkill schema-registry

      - name: Memory Profiling
        run: |
          heaptrack ./target/release/schema-registry-server &
          sleep 10
          k6 run tests/load/stress_test.js
          pkill schema-registry

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: profiling-results
          path: |
            flamegraph.svg
            heaptrack.*.gz
```

### Production Profiling

For live production profiling (minimal overhead):

```rust
// Enable pprof endpoint
use pprof::protos::Message;

#[get("/debug/pprof/profile")]
async fn cpu_profile(duration: Option<u64>) -> Result<Vec<u8>> {
    let duration = duration.unwrap_or(30);

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(99)
        .blocklist(&["libc", "libgcc", "pthread"])
        .build()
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;

    let report = guard.report().build().unwrap();
    let mut body = Vec::new();
    report.pprof().unwrap().encode(&mut body).unwrap();

    Ok(body)
}

#[get("/debug/pprof/heap")]
async fn heap_profile() -> Result<String> {
    // Heap profiling implementation
    Ok("Heap profile data".to_string())
}
```

## Best Practices

### Before Profiling

1. Build with optimizations: `cargo build --release`
2. Disable debug assertions: Remove `debug-assertions = true`
3. Use production-like data: Real schema sizes, realistic traffic
4. Warm up the system: Run load for 5 minutes before profiling
5. Profile under stable load: Not during ramp-up or ramp-down

### During Profiling

1. Run sufficient duration: 30-60 seconds minimum
2. Monitor system resources: Ensure no other interference
3. Use representative workload: Match production patterns
4. Profile both hot and cold paths: Include cache misses
5. Test edge cases: Large schemas, high concurrency

### After Profiling

1. Compare baseline: Track changes over time
2. Set performance budgets: Max CPU/memory per operation
3. Document findings: Keep profiling reports in git
4. Create benchmarks: Prevent regressions
5. Iterate: Profile after optimizations

## Common Performance Issues

### Issue: High CPU in JSON Serialization

**Symptoms**: `serde_json` consuming >20% CPU

**Solutions**:
- Use `simd-json` for faster parsing
- Cache serialized responses
- Use binary formats (bincode, msgpack) for internal APIs
- Implement custom serializers for hot paths

### Issue: Memory Growth Under Load

**Symptoms**: Heap usage increases linearly with requests

**Solutions**:
- Check for leaked `Arc` references
- Profile with heaptrack to find allocation hot spots
- Use object pools for frequently allocated types
- Ensure async tasks complete (no orphaned futures)

### Issue: Lock Contention

**Symptoms**: High CPU in `RwLock::write` or `Mutex::lock`

**Solutions**:
- Use `Arc<RwLock<T>>` for read-heavy workloads
- Consider lock-free data structures (`crossbeam`)
- Reduce critical section size
- Shard locks across multiple instances

### Issue: Slow Database Queries

**Symptoms**: >50ms per query in profiles

**Solutions**:
- Add database indexes (see `002_performance_indexes.sql`)
- Use prepared statements
- Implement query caching
- Batch multiple queries
- Use connection pooling

## Resources

- [Flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [Heaptrack](https://github.com/KDE/heaptrack)
- [Valgrind](https://valgrind.org/)
- [perf](https://perf.wiki.kernel.org/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [pprof-rs](https://github.com/tikv/pprof-rs)
