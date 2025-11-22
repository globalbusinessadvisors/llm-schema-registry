# Chaos Engineering Tests

This directory contains Chaos Mesh manifests for testing the LLM Schema Registry's resilience under various failure scenarios.

## Scenarios

### 1. Pod Failure (`pod-failure.yaml`)
- **What it tests**: Pod crash recovery
- **Frequency**: Every 15 minutes
- **Duration**: 60 seconds
- **Expected behavior**: Service should remain available with minimal disruption

### 2. Network Latency (`network-latency.yaml`)
- **What it tests**: Performance under degraded network conditions
- **Latency**: 100ms ± 50ms jitter
- **Duration**: 5 minutes
- **Expected behavior**: Requests should complete but with higher latency

### 3. Network Partition (`network-partition.yaml`)
- **What it tests**: Split-brain scenarios
- **Duration**: 2 minutes
- **Trigger**: Manual only
- **Expected behavior**: Service should detect partition and handle gracefully

### 4. Resource Stress (`resource-stress.yaml`)
- **What it tests**: Resource exhaustion handling
- **CPU Load**: 80%
- **Memory**: 256MB stress
- **Expected behavior**: Service should throttle gracefully, not crash

### 5. Database Failure (`database-failure.yaml`)
- **What it tests**: Database connectivity issues
- **Packet Loss**: 50%
- **Trigger**: Manual only
- **Expected behavior**: Circuit breaker should activate, cached data served

## Running Chaos Tests

### Prerequisites
```bash
# Install Chaos Mesh
kubectl create ns chaos-mesh
helm repo add chaos-mesh https://charts.chaos-mesh.org
helm install chaos-mesh chaos-mesh/chaos-mesh -n=chaos-mesh --version 2.6.0
```

### Apply Chaos Scenarios
```bash
# Apply all chaos scenarios
kubectl apply -f tests/chaos/

# Apply specific scenario
kubectl apply -f tests/chaos/pod-failure.yaml

# Check status
kubectl get podchaos -n schema-registry

# Delete all chaos scenarios
kubectl delete -f tests/chaos/
```

### Manual Testing
```bash
# Trigger pod failure manually
kubectl delete -f tests/chaos/pod-failure.yaml
kubectl apply -f tests/chaos/pod-failure.yaml

# Watch pods during chaos
kubectl get pods -n schema-registry -w

# Check logs
kubectl logs -n schema-registry -l app=schema-registry --tail=100
```

## Success Criteria

1. **Availability**: >99% uptime during chaos tests
2. **Error Rate**: <1% increase in error rate
3. **Latency**: p95 latency <50ms (2x normal under stress)
4. **Recovery Time**: <30 seconds after chaos ends

## Integration with Load Tests

Run chaos tests during load tests for comprehensive resilience validation:

```bash
# Terminal 1: Start load test
k6 run tests/load/basic_load.js

# Terminal 2: Apply chaos
kubectl apply -f tests/chaos/pod-failure.yaml

# Monitor metrics in Grafana
```
