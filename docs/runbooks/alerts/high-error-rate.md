# Alert Runbook: High Error Rate

**Alert Name:** `HighErrorRate`
**Severity:** Critical (P0)
**Alert Threshold:** Error rate > 5% for 5 minutes
**Auto-Resolution:** When error rate < 1% for 10 minutes

---

## Alert Description

This alert fires when the HTTP error rate (5xx responses) exceeds 5% over a 5-minute window. This indicates a significant service degradation that requires immediate attention.

## Immediate Actions (< 5 minutes)

### 1. Acknowledge Alert
```bash
# Acknowledge in PagerDuty
# Add note: "Investigating high error rate"
```

### 2. Check Current Status
```bash
# View current error rate
curl https://schema-registry.example.com/metrics | grep http_requests_total

# Check recent logs
kubectl logs -n schema-registry -l app=schema-registry \
  --tail=100 \
  --since=10m | grep ERROR

# Open Grafana dashboard
# https://grafana.example.com/d/schema-registry/errors
```

### 3. Quick Impact Assessment
- How many requests are failing?
- What percentage of traffic?
- Which endpoints are affected?
- Are errors affecting all users or specific tenants?

## Investigation Steps

### 1. Identify Error Patterns

```bash
# Get error breakdown by endpoint
kubectl logs -n schema-registry -l app=schema-registry \
  --since=15m | \
  grep "status=5" | \
  awk '{print $7}' | \
  sort | uniq -c | sort -rn

# Get error types
kubectl logs -n schema-registry -l app=schema-registry \
  --since=15m | \
  grep ERROR | \
  awk -F'ERROR:' '{print $2}' | \
  awk '{print $1}' | \
  sort | uniq -c | sort -rn
```

### 2. Check Dependencies

```bash
# Database connectivity
kubectl exec -n schema-registry \
  $(kubectl get pod -n schema-registry -l app=schema-registry -o jsonpath='{.items[0].metadata.name}') \
  -- psql $DATABASE_URL -c "SELECT 1"

# Redis connectivity
kubectl exec -n schema-registry \
  $(kubectl get pod -n schema-registry -l app=schema-registry -o jsonpath='{.items[0].metadata.name}') \
  -- redis-cli -h $REDIS_HOST PING

# S3 access
aws s3 ls s3://schema-registry-backups/
```

### 3. Check Resource Constraints

```bash
# Pod resource usage
kubectl top pods -n schema-registry -l app=schema-registry

# Check for OOMKilled pods
kubectl get pods -n schema-registry -l app=schema-registry \
  -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.status.containerStatuses[0].lastState.terminated.reason}{"\n"}{end}'

# Check for CPU throttling
kubectl exec -n schema-registry \
  $(kubectl get pod -n schema-registry -l app=schema-registry -o jsonpath='{.items[0].metadata.name}') \
  -- cat /sys/fs/cgroup/cpu/cpu.stat
```

## Common Causes & Solutions

### Cause 1: Database Connection Pool Exhausted

**Symptoms:**
- Errors: "connection pool timeout"
- Database connections at max

**Solution:**
```bash
# Increase pool size temporarily
kubectl set env deployment/schema-registry -n schema-registry \
  DATABASE_POOL_SIZE=150

# Or restart pods to clear stuck connections
kubectl rollout restart deployment/schema-registry -n schema-registry
```

### Cause 2: Database Performance Issues

**Symptoms:**
- Slow query logs
- High database CPU
- Timeouts on database operations

**Solution:**
```bash
# Check slow queries
psql $DATABASE_URL -c "SELECT pid, now() - query_start AS duration, query FROM pg_stat_activity WHERE state = 'active' ORDER BY duration DESC LIMIT 10;"

# Kill long-running queries if needed
psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 minutes';"

# Run VACUUM ANALYZE
./scripts/db-maintenance.sh --vacuum
```

### Cause 3: Redis Connection Issues

**Symptoms:**
- Errors: "redis connection failed"
- Cache timeouts

**Solution:**
```bash
# Check Redis health
kubectl exec -n schema-registry svc/redis -- redis-cli INFO

# Clear cache if corrupted
kubectl exec -n schema-registry svc/redis -- redis-cli FLUSHALL

# Restart Redis if needed
kubectl rollout restart statefulset/redis -n schema-registry
```

### Cause 4: Recent Deployment Issues

**Symptoms:**
- Errors started after recent deployment
- New error patterns not seen before

**Solution:**
```bash
# Rollback to previous version
helm rollback schema-registry -n schema-registry --wait

# See rollback runbook for details
# docs/runbooks/operations/rollback.md
```

### Cause 5: Memory Leaks / Resource Exhaustion

**Symptoms:**
- Increasing memory usage over time
- OOMKilled pods
- Swap usage increasing

**Solution:**
```bash
# Restart all pods (rolling restart)
kubectl rollout restart deployment/schema-registry -n schema-registry

# Increase memory limits
# See scaling runbook: docs/runbooks/operations/scaling.md

# Enable memory profiling
kubectl set env deployment/schema-registry -n schema-registry \
  MEMORY_PROFILING=enabled
```

### Cause 6: External Traffic Spike / DDoS

**Symptoms:**
- Sudden traffic increase
- Requests from unusual sources
- All pods maxed out on CPU

**Solution:**
```bash
# Enable rate limiting
kubectl set env deployment/schema-registry -n schema-registry \
  RATE_LIMIT_ENABLED=true \
  RATE_LIMIT_REQUESTS_PER_SECOND=100

# Scale up horizontally
kubectl scale deployment schema-registry -n schema-registry --replicas=10

# Block malicious IPs at ingress
kubectl edit ingress schema-registry -n schema-registry
# Add IP whitelist/blacklist
```

## Mitigation Steps

If errors persist after investigation:

### 1. Temporary Service Degradation

```bash
# Disable expensive endpoints temporarily
kubectl set env deployment/schema-registry -n schema-registry \
  FEATURE_FLAG_SEARCH_DISABLED=true

# Or enable read-only mode
kubectl set env deployment/schema-registry -n schema-registry \
  READ_ONLY_MODE=true
```

### 2. Circuit Breaker

```bash
# Enable circuit breaker for external dependencies
kubectl set env deployment/schema-registry -n schema-registry \
  CIRCUIT_BREAKER_ENABLED=true \
  CIRCUIT_BREAKER_THRESHOLD=50
```

### 3. Emergency Rollback

```bash
# See rollback runbook
./docs/runbooks/operations/rollback.md
```

## Communication

### Update Status Page
```
Title: Elevated Error Rates
Status: Investigating / Identified / Monitoring
Impact: Some requests may be failing (X% error rate)
Updates: Will update every 15 minutes
```

### Stakeholder Notification
```
To: #incidents, engineering-leadership
Subject: P0 Incident - High Error Rate

Current Status: Investigating
Error Rate: 8.5% (threshold: 5%)
Impact: Affecting ~500 requests/minute
Actions Taken:
  - Investigating root cause
  - Increased database pool size
  - Scaled to 8 replicas
Next Update: 15 minutes
```

## Resolution & Follow-up

### 1. Verify Resolution

```bash
# Check error rate back to normal
curl https://schema-registry.example.com/metrics | \
  grep http_requests_total | grep "status=\"5"

# Monitor for 30 minutes
# Open Grafana and watch dashboard
```

### 2. Root Cause Analysis

- What caused the errors?
- Why did monitoring/alerting catch it?
- What was the customer impact?
- How can we prevent this in the future?

### 3. Post-Incident Tasks

- [ ] Update status page to "Resolved"
- [ ] Schedule post-incident review meeting
- [ ] Document timeline in incident ticket
- [ ] Create action items for prevention
- [ ] Update runbook with learnings

## Escalation Path

- **0-15 minutes:** On-call SRE investigates
- **15-30 minutes:** Escalate to Engineering Team Lead
- **30-60 minutes:** Escalate to Engineering Manager
- **60+ minutes or data loss:** Escalate to CTO

## Related Runbooks

- [Rollback Procedure](../operations/rollback.md)
- [Database Performance](../operations/database-performance.md)
- [Scaling](../operations/scaling.md)
- [Incident Response](../incidents/incident-response.md)

## Metrics to Monitor

```promql
# Error rate
sum(rate(schema_registry_http_requests_total{status=~"5.."}[5m]))
/
sum(rate(schema_registry_http_requests_total[5m]))

# Request volume
sum(rate(schema_registry_http_requests_total[5m]))

# Latency
histogram_quantile(0.95, rate(schema_registry_http_request_duration_seconds_bucket[5m]))

# Database pool utilization
schema_registry_db_connections_active / schema_registry_db_connections_max
```

---

**Last Updated:** 2025-11-22
**Reviewed By:** SRE Team
**Next Review:** 2026-02-22
