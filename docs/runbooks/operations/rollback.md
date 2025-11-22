# Runbook: Emergency Rollback

**Severity:** Critical
**Estimated Time:** 5-10 minutes
**Prerequisite Skills:** Kubernetes, Helm
**On-Call Escalation:** Immediate - Page On-Call SRE

---

## Overview

Emergency rollback procedure for reverting to a previous stable version of the Schema Registry.

## Rollback Triggers

Execute immediate rollback if any of the following occur:

- Error rate > 10% for 5+ minutes
- Critical data corruption detected
- Database migration failures
- Service completely unavailable
- Security vulnerability actively exploited
- p99 latency > 1000ms for 10+ minutes

## Quick Rollback (< 2 minutes)

```bash
# Rollback to previous Helm release
helm rollback schema-registry -n schema-registry --wait

# Verify status
kubectl rollout status deployment/schema-registry -n schema-registry

# Quick health check
curl https://schema-registry.example.com/health
```

## Detailed Rollback Procedure

### 1. Assess Situation

```bash
# Check current deployment status
kubectl get pods -n schema-registry -l app=schema-registry

# View recent errors
kubectl logs -n schema-registry -l app=schema-registry --tail=500 | grep ERROR

# Check metrics dashboard
# Open: https://grafana.example.com/d/schema-registry
```

### 2. Notify Stakeholders

```bash
# Post to incident channel
# Slack: #incidents
# Message: "Schema Registry rollback in progress - <reason>"

# Update status page
# https://status.example.com - Set to "Degraded" or "Major Outage"
```

### 3. Execute Rollback

```bash
# List recent releases
helm history schema-registry -n schema-registry

# Rollback to specific version
helm rollback schema-registry <revision> -n schema-registry --wait --timeout=5m

# Monitor rollout
watch kubectl get pods -n schema-registry -l app=schema-registry
```

### 4. Verify Rollback Success

```bash
# Check all pods running
kubectl get pods -n schema-registry

# Verify correct version
kubectl get deployment schema-registry -n schema-registry \
  -o jsonpath='{.spec.template.spec.containers[0].image}'

# Run smoke tests
./scripts/smoke-test.sh production

# Check error rates
# Expected: < 1% within 5 minutes
```

### 5. Database Rollback (if needed)

```bash
# Only if database migration was part of failed deployment
# WARNING: May result in data loss

# Connect to database
kubectl port-forward -n schema-registry svc/postgresql 5432:5432

# Run migration rollback (if using flyway/sqlx)
psql $DATABASE_URL -c "DELETE FROM schema_migrations WHERE version = 'X';"

# Verify schema version
psql $DATABASE_URL -c "SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 5;"
```

### 6. Clear Caches

```bash
# Clear Redis cache to ensure consistency
kubectl exec -n schema-registry svc/redis -- redis-cli FLUSHALL

# Restart pods to rebuild cache
kubectl rollout restart deployment/schema-registry -n schema-registry
```

### 7. Post-Rollback Monitoring

Monitor for 30 minutes:
- Error rates return to baseline (< 1%)
- Latency metrics normal (p95 < 10ms)
- No new critical alerts
- Cache hit rate recovers to > 95%

## Rollback with Database Issues

If database migration cannot be rolled back:

```bash
# Option 1: Restore from backup (RPO < 1 hour)
./scripts/disaster-recovery.sh <backup-id>

# Option 2: Manual data fixes (if possible)
psql $DATABASE_URL -f fixes/manual-migration-fix.sql
```

## Communication Template

**Initial Notification:**
```
INCIDENT: Schema Registry Rollback
Severity: P1
Status: In Progress
Impact: [Describe impact]
Action: Rolling back to version X.Y.Z
ETA: 10 minutes
Updates: Every 5 minutes
```

**Resolution Notification:**
```
RESOLVED: Schema Registry Rollback Complete
Duration: [X minutes]
Root Cause: [Brief description]
Action Taken: Rolled back from v1.2.3 to v1.2.2
Next Steps: Post-incident review scheduled
Status Page: Updated to Operational
```

## Troubleshooting

### Rollback Fails

```bash
# Force delete problematic pods
kubectl delete pod -n schema-registry -l app=schema-registry --force

# Scale down to 0 and back up
kubectl scale deployment schema-registry -n schema-registry --replicas=0
kubectl scale deployment schema-registry -n schema-registry --replicas=3

# Nuclear option: Delete and reinstall
helm uninstall schema-registry -n schema-registry
helm install schema-registry ./helm/schema-registry -n schema-registry -f values-prod.yaml
```

### Database in Inconsistent State

```bash
# Restore from most recent backup
./scripts/disaster-recovery.sh $(./scripts/find-latest-backup.sh)

# Verify data integrity
psql $DATABASE_URL -f scripts/verify-data-integrity.sql
```

## Post-Rollback Actions

- [ ] Update incident ticket with timeline
- [ ] Schedule post-incident review within 24 hours
- [ ] Update status page to "Operational"
- [ ] Notify all stakeholders
- [ ] Block problematic version from deployment
- [ ] Create hotfix plan if needed
- [ ] Update monitoring/alerting if gaps found

## Escalation

- **Rollback fails after 2 attempts:** Escalate to Engineering Manager
- **Data corruption suspected:** Escalate to CTO immediately
- **Customer-facing impact:** Notify Customer Support & Product teams

## Related Runbooks

- [Deployment](deployment.md)
- [Disaster Recovery](../incidents/disaster-recovery-full.md)
- [Database Migration](database-migration.md)

---

**Last Updated:** 2025-11-22
**Review Frequency:** Quarterly
