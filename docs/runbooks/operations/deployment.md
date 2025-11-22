# Runbook: Production Deployment

**Severity:** Standard
**Estimated Time:** 15-20 minutes
**Prerequisite Skills:** Kubernetes, Helm
**On-Call Escalation:** DevOps Team Lead

---

## Overview

This runbook covers the standard procedure for deploying the LLM Schema Registry to production environments using Helm.

## Pre-Deployment Checklist

- [ ] All CI/CD tests passing (unit, integration, E2E)
- [ ] Security scan completed with no critical issues
- [ ] Performance benchmarks meet SLA targets
- [ ] Database migrations reviewed and tested in staging
- [ ] Rollback plan documented
- [ ] Change request approved (if required)
- [ ] Stakeholders notified of deployment window
- [ ] Monitoring dashboards open and ready

## Prerequisites

- kubectl configured with production cluster access
- Helm 3.x installed
- Docker image built and pushed to registry
- Access to deployment repository
- PagerDuty/alerting configured

## Step-by-Step Procedure

### 1. Pre-Deployment Validation

```bash
# Verify cluster connectivity
kubectl cluster-info

# Check current deployment status
kubectl get deployments -n schema-registry

# Verify current version
kubectl get deployment schema-registry -n schema-registry \
  -o jsonpath='{.spec.template.spec.containers[0].image}'

# Check resource availability
kubectl top nodes
```

### 2. Create Backup

```bash
# Trigger manual backup before deployment
kubectl exec -n schema-registry \
  $(kubectl get pod -n schema-registry -l app=schema-registry -o jsonpath='{.items[0].metadata.name}') \
  -- /usr/local/bin/backup-runner --manual

# Verify backup completed
aws s3 ls s3://schema-registry-backups/backups/manual/
```

### 3. Update Helm Values

```bash
# Update image tag in values file
cd helm/schema-registry

# Edit values-production.yaml
vim values-production.yaml

# Verify changes
git diff values-production.yaml
```

### 4. Run Helm Upgrade (Dry Run)

```bash
# Perform dry-run to validate changes
helm upgrade schema-registry . \
  -n schema-registry \
  -f values-production.yaml \
  --set image.tag=v1.2.3 \
  --dry-run --debug

# Review output for any issues
```

### 5. Execute Deployment

```bash
# Deploy with rolling update strategy
helm upgrade schema-registry . \
  -n schema-registry \
  -f values-production.yaml \
  --set image.tag=v1.2.3 \
  --wait \
  --timeout=10m

# Monitor rollout status
kubectl rollout status deployment/schema-registry -n schema-registry
```

### 6. Post-Deployment Validation

```bash
# Check pod status
kubectl get pods -n schema-registry -l app=schema-registry

# View recent logs
kubectl logs -n schema-registry -l app=schema-registry --tail=100

# Run smoke tests
./scripts/smoke-test.sh

# Check health endpoint
curl https://schema-registry.example.com/health

# Verify metrics
curl https://schema-registry.example.com/metrics | grep schema_registry_http_requests_total
```

### 7. Monitor for Issues

- Open Grafana dashboard: [Schema Registry Overview](https://grafana.example.com/d/schema-registry)
- Monitor error rates for 15 minutes
- Watch latency metrics (p95, p99)
- Check Sentry/error tracking for new errors
- Monitor PagerDuty for alerts

**Success Criteria:**
- Error rate < 1%
- p95 latency < 10ms (retrieval), < 100ms (registration)
- No critical alerts fired
- All pods in Running state

### 8. Update Documentation

```bash
# Record deployment in changelog
echo "$(date): Deployed v1.2.3 to production" >> CHANGELOG.md

# Update runbook if procedures changed
git add docs/runbooks/
git commit -m "docs: Update deployment runbook post v1.2.3"
```

## Rollback Procedure

If issues are detected, rollback immediately:

```bash
# Rollback to previous version
helm rollback schema-registry -n schema-registry

# Or deploy specific revision
helm rollback schema-registry 5 -n schema-registry

# Verify rollback
kubectl rollout status deployment/schema-registry -n schema-registry

# Run smoke tests
./scripts/smoke-test.sh
```

**Rollback Triggers:**
- Error rate > 5% for 5 minutes
- p95 latency > 50ms for 10 minutes
- Critical alerts firing
- Database migration failures
- Pod crashloops

## Troubleshooting

### Pods Not Starting

```bash
# Describe pod to see events
kubectl describe pod -n schema-registry <pod-name>

# Check pod logs
kubectl logs -n schema-registry <pod-name>

# Common issues:
# - ImagePullBackOff: Verify image exists in registry
# - CrashLoopBackOff: Check application logs
# - Pending: Check resource constraints
```

### Database Connection Issues

```bash
# Verify database connectivity from pod
kubectl exec -n schema-registry <pod-name> -- \
  psql $DATABASE_URL -c "SELECT 1"

# Check database credentials secret
kubectl get secret schema-registry-db -n schema-registry -o yaml
```

### High Error Rate After Deployment

```bash
# Check error logs
kubectl logs -n schema-registry -l app=schema-registry | grep ERROR

# Check Sentry for error patterns
# Open Grafana error dashboard

# If database migration issue, may need to:
# 1. Rollback deployment
# 2. Fix migration
# 3. Re-deploy
```

## Escalation

- **Initial Deployment Issues:** Contact DevOps Team Lead
- **Database Issues:** Contact Database Administrator
- **Persistent Issues (>30 min):** Escalate to Engineering Manager
- **Data Corruption:** Immediately escalate to CTO

## Post-Deployment

- [ ] Update status page
- [ ] Notify stakeholders of successful deployment
- [ ] Schedule post-deployment review (if major release)
- [ ] Update capacity planning metrics
- [ ] Close change request ticket

## Related Runbooks

- [Rollback Procedure](rollback.md)
- [Database Migration](database-migration.md)
- [Incident Response](../incidents/incident-response.md)

## Revision History

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-11-22 | 1.0 | SRE Team | Initial version |
