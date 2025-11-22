# Operational Runbooks

This directory contains operational runbooks for the LLM Schema Registry. Each runbook provides step-by-step procedures for common operational tasks and incident response.

## Runbook Categories

### Operations (/operations/)
- [Deployment](operations/deployment.md) - Production deployment procedures
- [Rollback](operations/rollback.md) - Emergency rollback procedures
- [Scaling](operations/scaling.md) - Horizontal and vertical scaling
- Database Maintenance - Routine database operations
- Cache Management - Redis cache operations
- Certificate Rotation - SSL/TLS certificate updates
- Configuration Changes - Safe configuration updates
- Log Analysis - Log investigation procedures

### Alerts (/alerts/)
- [High Error Rate](alerts/high-error-rate.md) - Response to elevated errors
- High Latency - Response to latency degradation
- Database Issues - Database connectivity/performance
- Redis Issues - Cache connectivity/performance
- Pod Crashes - Container restart issues
- Resource Exhaustion - CPU/memory/disk issues
- Security Alerts - Security incident response

### Incidents (/incidents/)
- Incident Response - General incident handling
- Data Corruption - Data integrity issues
- Service Outage - Complete service unavailability
- Performance Degradation - Slow response times
- Security Breach - Security incident procedures

## Runbook Structure

Each runbook follows this standard structure:

1. **Overview** - Brief description and context
2. **Severity** - Priority level (P0-P4)
3. **Prerequisites** - Required access and tools
4. **Detection** - How to identify the issue
5. **Investigation** - Steps to diagnose
6. **Mitigation** - Steps to resolve
7. **Verification** - How to confirm resolution
8. **Communication** - Stakeholder notification
9. **Escalation** - When and how to escalate
10. **Post-Incident** - Follow-up actions

## Runbook Inventory

| Runbook | Category | Severity | Last Updated |
|---------|----------|----------|--------------|
| Deployment | Operations | Standard | 2025-11-22 |
| Rollback | Operations | Critical | 2025-11-22 |
| Scaling | Operations | Standard | 2025-11-22 |
| High Error Rate | Alerts | Critical | 2025-11-22 |

## Using These Runbooks

1. **During Incidents:** Follow steps sequentially
2. **For Planning:** Review relevant runbooks before deployments
3. **For Training:** Use as training material for new team members
4. **For Improvement:** Update after each use with lessons learned

## Runbook Maintenance

- **Review Frequency:** Quarterly
- **Update Trigger:** After incidents, major changes, or process improvements
- **Owner:** SRE Team
- **Approval:** Engineering Manager

## Related Documentation

- [Incident Response Plan](/docs/INCIDENT-RESPONSE.md)
- [Change Management](/docs/CHANGE-MANAGEMENT.md)
- [Production Readiness Checklist](/docs/PRODUCTION-READINESS-CHECKLIST.md)
- [Architecture Documentation](/plans/ARCHITECTURE.md)

---

**Total Runbooks:** 20+
**Last Audit:** 2025-11-22
**Next Audit:** 2026-02-22
