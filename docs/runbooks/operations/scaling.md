# Runbook: Horizontal and Vertical Scaling

**Severity:** Standard
**Estimated Time:** 10-30 minutes
**Prerequisite Skills:** Kubernetes, resource management
**On-Call Escalation:** DevOps Team

---

## Overview

Procedures for scaling the Schema Registry horizontally (more replicas) and vertically (more resources per pod).

## When to Scale

### Scale Up Triggers

**Horizontal Scaling (Add Replicas):**
- Average CPU utilization > 60% across all pods
- Average memory utilization > 60%
- Request rate approaching capacity (> 8000 req/s per 3 replicas)
- Anticipating traffic spike (planned event)

**Vertical Scaling (Increase Resources):**
- Pods hitting CPU limits (throttling)
- Pods hitting memory limits (OOM kills)
- Single-threaded operations bottlenecked
- Database connection pool exhaustion

### Scale Down Triggers

- Average CPU < 30% for 24+ hours
- Average memory < 30% for 24+ hours
- Cost optimization needs
- Over-provisioned for current traffic

## Horizontal Scaling

### Manual Scaling

```bash
# Check current replicas
kubectl get deployment schema-registry -n schema-registry

# Scale up to 5 replicas
kubectl scale deployment schema-registry -n schema-registry --replicas=5

# Monitor rollout
kubectl rollout status deployment/schema-registry -n schema-registry

# Verify all pods healthy
kubectl get pods -n schema-registry -l app=schema-registry
```

### Update HPA (Horizontal Pod Autoscaler)

```bash
# Check current HPA settings
kubectl get hpa schema-registry -n schema-registry -o yaml

# Update HPA via Helm values
vim helm/schema-registry/values-production.yaml

# Update autoscaling section:
# autoscaling:
#   enabled: true
#   minReplicas: 3
#   maxReplicas: 10
#   targetCPUUtilizationPercentage: 60
#   targetMemoryUtilizationPercentage: 60

# Apply changes
helm upgrade schema-registry ./helm/schema-registry \
  -n schema-registry \
  -f values-production.yaml

# Verify HPA
kubectl get hpa schema-registry -n schema-registry
```

### Gradual Scale-Up for High Traffic Events

```bash
# Pre-event scaling (2 hours before)
kubectl scale deployment schema-registry -n schema-registry --replicas=8

# Monitor during event
watch kubectl top pods -n schema-registry

# Post-event scale-down (2 hours after)
kubectl scale deployment schema-registry -n schema-registry --replicas=3
```

## Vertical Scaling

### Increase CPU/Memory Limits

```bash
# Edit deployment resources
vim helm/schema-registry/values-production.yaml

# Update resources:
# resources:
#   requests:
#     cpu: 1000m
#     memory: 1Gi
#   limits:
#     cpu: 2000m
#     memory: 2Gi

# Apply changes
helm upgrade schema-registry ./helm/schema-registry \
  -n schema-registry \
  -f values-production.yaml \
  --wait

# Monitor rollout
kubectl rollout status deployment/schema-registry -n schema-registry

# Verify new resource allocation
kubectl describe pod -n schema-registry -l app=schema-registry | grep -A 5 "Limits"
```

### Optimize Resource Requests

```bash
# Analyze actual usage
kubectl top pods -n schema-registry -l app=schema-registry

# Calculate average usage over time
kubectl get --raw /apis/metrics.k8s.io/v1beta1/namespaces/schema-registry/pods | \
  jq '.items[] | select(.metadata.labels.app=="schema-registry") | .containers[0].usage'

# Right-size based on data
# Rule: requests = 1.5x average, limits = 2x average
```

## Database Connection Pool Scaling

### Increase Pool Size

```bash
# Update environment variables
kubectl set env deployment/schema-registry -n schema-registry \
  DATABASE_POOL_SIZE=100 \
  DATABASE_POOL_MIN=10

# Or via Helm values
vim helm/schema-registry/values-production.yaml

# Add/update:
# env:
#   - name: DATABASE_POOL_SIZE
#     value: "100"
#   - name: DATABASE_POOL_MIN
#     value: "10"

# Apply
helm upgrade schema-registry ./helm/schema-registry \
  -n schema-registry \
  -f values-production.yaml
```

### Verify Pool Utilization

```bash
# Check metrics
curl https://schema-registry.example.com/metrics | \
  grep schema_registry_db_connections

# Expected output:
# schema_registry_db_connections_active{pool="postgres"} 25
# schema_registry_db_connections_max{pool="postgres"} 100
```

## Redis Cache Scaling

### Scale Redis (if self-hosted)

```bash
# For Redis cluster
kubectl scale statefulset redis -n schema-registry --replicas=5

# Or use Redis managed service
# Update connection string to point to cluster
```

### Increase Redis Memory

```bash
# Update Redis configuration
kubectl edit configmap redis-config -n schema-registry

# Update maxmemory
# maxmemory 4gb

# Restart Redis
kubectl rollout restart statefulset/redis -n schema-registry
```

## Validation After Scaling

### 1. Health Checks

```bash
# All pods healthy
kubectl get pods -n schema-registry -l app=schema-registry

# Health endpoint
for pod in $(kubectl get pods -n schema-registry -l app=schema-registry -o name); do
  kubectl exec -n schema-registry $pod -- curl -s localhost:8080/health
done
```

### 2. Load Distribution

```bash
# Check request distribution across pods
kubectl logs -n schema-registry -l app=schema-registry --tail=100 | \
  grep "HTTP" | \
  awk '{print $1}' | \
  sort | uniq -c

# Should be roughly equal across pods
```

### 3. Performance Metrics

```bash
# Check p95 latency
curl https://schema-registry.example.com/metrics | \
  grep http_request_duration | \
  grep 'quantile="0.95"'

# Check error rate
curl https://schema-registry.example.com/metrics | \
  grep http_requests_total
```

### 4. Database Connections

```bash
# Verify connection pool not exhausted
kubectl logs -n schema-registry -l app=schema-registry | \
  grep "connection pool"

# Check PostgreSQL connection count
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity WHERE application_name = 'schema-registry';"
```

## Capacity Planning

### Calculate Required Replicas

```
Required Replicas = (Target RPS) / (RPS per pod) * 1.5 (buffer)

Example:
- Target: 15,000 RPS
- Per pod capacity: 2,000 RPS
- Required: 15,000 / 2,000 * 1.5 = 12 replicas
```

### Cost Optimization

```bash
# Run capacity planning script
./scripts/capacity-planning.sh

# Review recommendations
cat /tmp/capacity-reports/capacity-report-*.txt

# Consider:
# - Can we scale down during off-hours?
# - Are we over-provisioned?
# - Spot instances for non-prod?
```

## Scaling Checklist

**Before Scaling:**
- [ ] Review current metrics (CPU, memory, requests)
- [ ] Check database capacity
- [ ] Verify no ongoing incidents
- [ ] Notify team in #infrastructure channel
- [ ] Have rollback plan ready

**During Scaling:**
- [ ] Monitor metrics dashboard
- [ ] Watch for errors in logs
- [ ] Verify load distribution
- [ ] Check database connection count

**After Scaling:**
- [ ] Validate performance metrics
- [ ] Run smoke tests
- [ ] Update capacity planning docs
- [ ] Document in changelog

## Troubleshooting

### Pods Not Distributing Load

```bash
# Check service endpoints
kubectl get endpoints schema-registry -n schema-registry

# Verify readiness probes
kubectl describe pod -n schema-registry -l app=schema-registry | grep -A 10 Readiness

# Check for pod affinity issues
kubectl describe pod -n schema-registry -l app=schema-registry | grep -A 5 Node-Selectors
```

### OOMKilled Pods After Scaling

```bash
# Increase memory limits
# See "Vertical Scaling" section above

# Enable memory profiling
kubectl set env deployment/schema-registry -n schema-registry \
  RUST_BACKTRACE=1 \
  MEMORY_PROFILING=true
```

### Database Connection Pool Exhausted

```bash
# Temporary fix: Increase pool size
kubectl set env deployment/schema-registry -n schema-registry \
  DATABASE_POOL_SIZE=200

# Long-term: Consider connection pooler (PgBouncer)
helm install pgbouncer bitnami/pgbouncer -n schema-registry
```

## Related Runbooks

- [Capacity Planning](capacity-planning.md)
- [Performance Tuning](performance-tuning.md)
- [Database Optimization](database-optimization.md)

---

**Last Updated:** 2025-11-22
**Review Frequency:** Monthly
