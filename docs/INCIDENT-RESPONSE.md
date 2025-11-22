# Incident Response Plan

## Overview

This document defines the incident response process for the LLM Schema Registry, including severity definitions, escalation paths, communication templates, and post-mortem procedures.

## Incident Severity Levels

### P0 - Critical

**Definition:** Complete service outage or data loss affecting all users

**Examples:**
- Service completely unavailable (all pods down)
- Data corruption or loss
- Security breach actively occurring
- Database completely unavailable

**Response Time:** Immediate (< 5 minutes)
**Resolution Target:** < 1 hour
**Escalation:** Page on-call SRE + Engineering Manager immediately
**Communication:** Every 15 minutes to all stakeholders

### P1 - High

**Definition:** Significant service degradation affecting majority of users

**Examples:**
- Error rate > 10%
- Latency > 10x normal
- Critical feature completely broken
- Partial data loss (recoverable from backup)

**Response Time:** < 15 minutes
**Resolution Target:** < 4 hours
**Escalation:** Page on-call SRE
**Communication:** Every 30 minutes to stakeholders

### P2 - Medium

**Definition:** Service degradation affecting subset of users or features

**Examples:**
- Error rate 5-10%
- Latency 3-10x normal
- Non-critical feature broken
- Performance degradation

**Response Time:** < 1 hour (business hours)
**Resolution Target:** < 24 hours
**Escalation:** Notify on-call SRE (no page)
**Communication:** Hourly updates

### P3 - Low

**Definition:** Minor issues with workarounds available

**Examples:**
- Error rate 1-5%
- Latency 2-3x normal
- Edge case bugs
- Non-urgent security vulnerabilities

**Response Time:** < 4 hours (business hours)
**Resolution Target:** < 1 week
**Escalation:** Create ticket for team
**Communication:** Daily updates

### P4 - Trivial

**Definition:** Cosmetic issues or feature requests

**Examples:**
- Documentation errors
- UI/UX improvements
- Performance optimizations (non-urgent)

**Response Time:** Best effort
**Resolution Target:** Backlog
**Escalation:** None
**Communication:** None required

## Incident Response Process

### 1. Detection & Initial Response (0-5 minutes)

#### Automated Detection
- PagerDuty alert fires
- Monitoring dashboard shows anomaly
- Sentry error spike

#### Manual Detection
- Customer report via support
- Team member notices issue
- External monitoring service alert

#### Initial Actions
```bash
# 1. Acknowledge incident
#    - PagerDuty: Acknowledge alert
#    - Slack: Post in #incidents channel

# 2. Create incident ticket
#    - Jira/Linear: Create incident ticket
#    - Template: Use incident template

# 3. Assess severity
#    - Review metrics dashboards
#    - Check error rates, latency
#    - Determine impact scope

# 4. Begin investigation
#    - Check recent changes
#    - Review logs
#    - Check dependencies
```

### 2. Triage & Assessment (5-15 minutes)

```bash
# Quick health check
kubectl get pods -n schema-registry
curl https://schema-registry.example.com/health

# Check metrics
open https://grafana.example.com/d/schema-registry

# Review recent deployments
helm history schema-registry -n schema-registry

# Check logs
kubectl logs -n schema-registry -l app=schema-registry --tail=500

# Assess impact
# - How many users affected?
# - Which features broken?
# - Any data loss?
# - Revenue impact?
```

### 3. Communication (Ongoing)

#### Initial Notification Template

**Slack (#incidents):**
```
🚨 INCIDENT ALERT

Severity: [P0/P1/P2/P3/P4]
Status: Investigating
Component: Schema Registry
Impact: [Describe impact]
Started: [Time]
Responder: [Your name]
Ticket: [JIRA-XXX]

Symptoms:
- [Symptom 1]
- [Symptom 2]

Actions Taken:
- [Action 1]
- [Action 2]

Next Update: [Time]
```

**Status Page:**
```
Title: Schema Registry Service Degradation
Status: Investigating
Impact: [Describe user-facing impact]
Updates: Will be provided every [15/30/60] minutes
Started: [Time]
```

**Email to Stakeholders (P0/P1 only):**
```
Subject: [P0/P1] Schema Registry Incident - [Brief Description]

Severity: [P0/P1]
Status: Investigating
Component: LLM Schema Registry

Impact:
- [Describe business impact]
- Estimated affected users: [Number/Percentage]

Current Status:
[Brief description of situation]

Actions Taken:
- [Action 1]
- [Action 2]

Next Steps:
- [Next step 1]
- [Next step 2]

Next Update: [Time]
ETA to Resolution: [Estimate]

Incident Commander: [Name]
Incident Ticket: [URL]
```

### 4. Investigation & Mitigation (15 minutes - 4 hours)

#### Investigation Checklist
- [ ] Check recent deployments/changes
- [ ] Review error logs and stack traces
- [ ] Check resource utilization (CPU, memory, disk)
- [ ] Verify database health and connectivity
- [ ] Verify Redis cache health
- [ ] Check S3 accessibility
- [ ] Review network connectivity
- [ ] Check for external dependencies issues
- [ ] Review related Sentry errors
- [ ] Check for security events

#### Common Mitigation Actions

**Rollback Recent Changes:**
```bash
helm rollback schema-registry -n schema-registry
# See: docs/runbooks/operations/rollback.md
```

**Scale Resources:**
```bash
kubectl scale deployment schema-registry -n schema-registry --replicas=8
# See: docs/runbooks/operations/scaling.md
```

**Database Issues:**
```bash
# Restart connections
kubectl rollout restart deployment/schema-registry -n schema-registry

# Run maintenance
./scripts/db-maintenance.sh --vacuum
```

**Clear Cache:**
```bash
kubectl exec -n schema-registry svc/redis -- redis-cli FLUSHALL
```

**Enable Circuit Breakers:**
```bash
kubectl set env deployment/schema-registry -n schema-registry \
  CIRCUIT_BREAKER_ENABLED=true
```

### 5. Resolution (Variable)

```bash
# Verify metrics back to normal
# - Error rate < 1%
# - Latency at baseline
# - No active alerts

# Run smoke tests
./scripts/smoke-test.sh production

# Monitor for stability (30-60 minutes)
# Watch Grafana dashboards for regressions
```

### 6. Communication - Resolution

**Slack (#incidents):**
```
✅ INCIDENT RESOLVED

Severity: [P0/P1/P2/P3]
Component: Schema Registry
Duration: [Duration]
Root Cause: [Brief description]

Timeline:
- [Time]: Incident detected
- [Time]: Mitigation started
- [Time]: Service restored
- [Time]: Confirmed stable

Actions Taken:
- [Action 1]
- [Action 2]

Next Steps:
- Post-incident review scheduled for [Date/Time]
- Follow-up action items created

Incident Ticket: [JIRA-XXX]
```

**Status Page:**
```
Title: Schema Registry - Resolved
Status: Resolved
Resolution: [Brief description of fix]
Duration: [Duration]
Root Cause: [Brief description]
Follow-up: Post-incident review scheduled
```

### 7. Post-Incident Review

**Scheduled within 24-48 hours of P0/P1, within 1 week for P2/P3**

#### Agenda
1. Timeline review (5 minutes)
2. Root cause analysis (15 minutes)
3. What went well (10 minutes)
4. What went wrong (10 minutes)
5. Action items (15 minutes)
6. Lessons learned (5 minutes)

#### Post-Mortem Template

```markdown
# Post-Incident Review: [Incident Title]

**Incident ID:** [JIRA-XXX]
**Date:** [Date]
**Severity:** [P0/P1/P2]
**Duration:** [Duration]
**Impact:** [Description]

## Summary

[2-3 sentence summary of incident]

## Timeline

| Time | Event |
|------|-------|
| 14:32 | Alert fired: High error rate |
| 14:33 | On-call acknowledged |
| 14:37 | Root cause identified: Database connection pool exhausted |
| 14:40 | Mitigation: Increased pool size |
| 14:45 | Service restored |
| 15:15 | Monitoring confirmed stable |

## Root Cause

[Detailed explanation of root cause]

**Contributing Factors:**
- [Factor 1]
- [Factor 2]

## Impact Assessment

**User Impact:**
- Affected users: [Number/Percentage]
- Affected requests: [Number]
- Error rate: [Percentage]

**Business Impact:**
- Revenue impact: [Amount or N/A]
- SLA breach: [Yes/No]
- Customer escalations: [Number]

## What Went Well

- Quick detection (< 1 minute)
- Effective communication
- Team coordination

## What Went Wrong

- Late escalation to database team
- Monitoring gaps identified
- Rollback took longer than expected

## Action Items

| Item | Owner | Due Date | Priority |
|------|-------|----------|----------|
| Add database connection pool monitoring | @sre-team | 2025-12-01 | P0 |
| Update rollback runbook | @ops-team | 2025-11-30 | P1 |
| Implement auto-scaling for connection pool | @eng-team | 2025-12-15 | P1 |
| Add circuit breaker for database | @eng-team | 2026-01-15 | P2 |

## Lessons Learned

- [Lesson 1]
- [Lesson 2]

## Appendices

### Relevant Logs
[Link to log snippets]

### Metrics Screenshots
[Link to Grafana screenshots]

### Communication Log
[Link to Slack thread]
```

## Escalation Path

```
P0/P1 Incidents:
┌──────────────────┐
│   Alert Fires    │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  On-Call SRE     │ ◄─── 0-5 minutes
└────────┬─────────┘
         │
         ├─(15 min)─► ┌──────────────────┐
         │            │ Engineering Lead │
         │            └──────────────────┘
         │
         ├─(30 min)─► ┌──────────────────┐
         │            │ Engineering Mgr  │
         │            └──────────────────┘
         │
         └─(60 min)─► ┌──────────────────┐
                      │      CTO         │
                      └──────────────────┘
```

## On-Call Rotation

**Primary On-Call:**
- Week 1: Engineer A
- Week 2: Engineer B
- Week 3: Engineer C
- Week 4: Engineer D

**Secondary On-Call (Backup):**
- SRE Team Lead

**Escalation:**
- Engineering Manager
- CTO (for P0 only)

## Communication Channels

**Primary:**
- Slack: #incidents
- PagerDuty: schema-registry-oncall
- Email: oncall-schema-registry@example.com

**Status Updates:**
- Status Page: https://status.example.com
- Twitter: @ExampleStatus (for major outages)

**Stakeholders:**
- Engineering: #engineering
- Product: #product
- Support: #customer-support
- Leadership: #leadership

## Blameless Culture

**Core Principles:**
1. **No Blame:** Focus on systems, not individuals
2. **Learning:** Every incident is a learning opportunity
3. **Improvement:** Action items to prevent recurrence
4. **Transparency:** Share learnings across organization

**Encouraged Behaviors:**
- Asking "why" repeatedly to find root cause
- Documenting everything
- Sharing learnings openly
- Taking ownership of action items

**Discouraged Behaviors:**
- Pointing fingers at individuals
- Hiding mistakes
- Skipping post-mortems
- Not following up on action items

## Tools & Resources

**Monitoring:**
- Grafana: https://grafana.example.com
- Prometheus: https://prometheus.example.com
- Sentry: https://sentry.io/schema-registry

**Incident Management:**
- PagerDuty: https://example.pagerduty.com
- Jira: https://example.atlassian.net
- Status Page: https://status.example.com

**Documentation:**
- Runbooks: /docs/runbooks/
- Architecture: /docs/ARCHITECTURE.md
- API Docs: https://api-docs.example.com

**Communication:**
- Slack: #incidents, #schema-registry
- Email: oncall-schema-registry@example.com

## Appendix: Quick Reference Commands

```bash
# Check service health
kubectl get pods -n schema-registry
curl https://schema-registry.example.com/health

# View logs
kubectl logs -n schema-registry -l app=schema-registry --tail=200 -f

# Check metrics
curl https://schema-registry.example.com/metrics | grep error

# Rollback deployment
helm rollback schema-registry -n schema-registry

# Scale up
kubectl scale deployment schema-registry -n schema-registry --replicas=8

# Restart pods
kubectl rollout restart deployment/schema-registry -n schema-registry

# Database health
psql $DATABASE_URL -c "SELECT 1"

# Redis health
redis-cli -h $REDIS_HOST PING

# Clear cache
kubectl exec -n schema-registry svc/redis -- redis-cli FLUSHALL
```

---

**Document Owner:** SRE Team
**Last Updated:** 2025-11-22
**Next Review:** 2026-01-22 (Quarterly)
**Version:** 1.0
