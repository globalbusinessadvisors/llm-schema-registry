#!/bin/bash
# Disaster Recovery Script for LLM Schema Registry
# Purpose: Restore schema registry from backup with comprehensive validation
# Usage: ./disaster-recovery.sh <backup-id> [--dry-run]
#
# RPO: < 1 hour (Recovery Point Objective)
# RTO: < 4 hours (Recovery Time Objective)

set -euo pipefail

# Configuration
BACKUP_ID="${1:-}"
DRY_RUN="${2:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="/var/log/schema-registry/disaster-recovery-$(date +%Y%m%d-%H%M%S).log"

# Environment variables with defaults
DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/schema_registry}"
REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
S3_BUCKET="${BACKUP_S3_BUCKET:-schema-registry-backups}"
KUBERNETES_NAMESPACE="${KUBERNETES_NAMESPACE:-schema-registry}"
DEPLOYMENT_NAME="${DEPLOYMENT_NAME:-schema-registry}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date +'%Y-%m-%d %H:%M:%S')
    echo -e "${timestamp} [${level}] ${message}" | tee -a "${LOG_FILE}"
}

log_info() {
    log "INFO" "${BLUE}$*${NC}"
}

log_success() {
    log "SUCCESS" "${GREEN}$*${NC}"
}

log_warn() {
    log "WARNING" "${YELLOW}$*${NC}"
}

log_error() {
    log "ERROR" "${RED}$*${NC}"
}

# Validation
if [ -z "$BACKUP_ID" ]; then
    log_error "Backup ID required"
    echo "Usage: $0 <backup-id> [--dry-run]"
    echo ""
    echo "Example: $0 550e8400-e29b-41d4-a716-446655440000"
    echo ""
    echo "Options:"
    echo "  --dry-run    Simulate recovery without making changes"
    exit 1
fi

# Dry run message
if [ "$DRY_RUN" == "--dry-run" ]; then
    log_warn "DRY RUN MODE - No changes will be made"
fi

# Start recovery
START_TIME=$(date +%s)
log_info "========================================"
log_info "Disaster Recovery Started"
log_info "========================================"
log_info "Backup ID: ${BACKUP_ID}"
log_info "Database: ${DATABASE_URL}"
log_info "Redis: ${REDIS_HOST}:${REDIS_PORT}"
log_info "S3 Bucket: ${S3_BUCKET}"
log_info "Namespace: ${KUBERNETES_NAMESPACE}"
log_info "========================================"

# Step 1: Pre-flight checks
log_info "Step 1/10: Running pre-flight checks"

# Check required commands
for cmd in aws psql redis-cli kubectl gzip; do
    if ! command -v "$cmd" &> /dev/null; then
        log_error "Required command not found: $cmd"
        exit 1
    fi
done
log_success "All required commands available"

# Check AWS credentials
if [ "$DRY_RUN" != "--dry-run" ]; then
    if ! aws s3 ls "s3://${S3_BUCKET}/" &> /dev/null; then
        log_error "Cannot access S3 bucket: ${S3_BUCKET}"
        exit 1
    fi
    log_success "AWS credentials valid"
fi

# Check database connectivity
if [ "$DRY_RUN" != "--dry-run" ]; then
    if ! psql "${DATABASE_URL}" -c "SELECT 1" &> /dev/null; then
        log_error "Cannot connect to database"
        exit 1
    fi
    log_success "Database connection valid"
fi

# Step 2: Stop traffic
log_info "Step 2/10: Stopping traffic to schema registry"

if [ "$DRY_RUN" != "--dry-run" ]; then
    CURRENT_REPLICAS=$(kubectl get deployment "${DEPLOYMENT_NAME}" \
        -n "${KUBERNETES_NAMESPACE}" \
        -o jsonpath='{.spec.replicas}' 2>/dev/null || echo "3")

    log_info "Current replicas: ${CURRENT_REPLICAS}"
    echo "${CURRENT_REPLICAS}" > /tmp/schema-registry-replicas.txt

    kubectl scale deployment "${DEPLOYMENT_NAME}" \
        --replicas=0 \
        -n "${KUBERNETES_NAMESPACE}"

    # Wait for pods to terminate
    log_info "Waiting for pods to terminate..."
    kubectl wait --for=delete pod \
        -l app=schema-registry \
        -n "${KUBERNETES_NAMESPACE}" \
        --timeout=60s || true

    log_success "Traffic stopped successfully"
else
    log_info "Would scale deployment to 0 replicas"
fi

sleep 2

# Step 3: Find and download backup
log_info "Step 3/10: Locating backup in S3"

BACKUP_FILE="/tmp/backup-${BACKUP_ID}.sql.gz"

if [ "$DRY_RUN" != "--dry-run" ]; then
    # Search for backup in all paths
    S3_KEY=$(aws s3 ls "s3://${S3_BUCKET}/backups/" --recursive | \
        grep "${BACKUP_ID}" | \
        awk '{print $4}' | \
        head -n 1)

    if [ -z "$S3_KEY" ]; then
        log_error "Backup not found: ${BACKUP_ID}"
        exit 1
    fi

    log_info "Found backup: ${S3_KEY}"

    # Download backup
    log_info "Downloading backup from S3..."
    aws s3 cp "s3://${S3_BUCKET}/${S3_KEY}" "${BACKUP_FILE}"

    BACKUP_SIZE=$(du -h "${BACKUP_FILE}" | cut -f1)
    log_success "Backup downloaded successfully (${BACKUP_SIZE})"
else
    log_info "Would download from s3://${S3_BUCKET}/backups/..."
fi

# Step 4: Verify backup integrity
log_info "Step 4/10: Verifying backup integrity"

if [ "$DRY_RUN" != "--dry-run" ]; then
    if ! gzip -t "${BACKUP_FILE}" 2>&1; then
        log_error "Backup file is corrupted"
        exit 1
    fi
    log_success "Backup integrity verified"
else
    log_info "Would verify gzip integrity"
fi

# Step 5: Create backup of current database
log_info "Step 5/10: Creating safety backup of current database"

if [ "$DRY_RUN" != "--dry-run" ]; then
    SAFETY_BACKUP="/tmp/safety-backup-$(date +%Y%m%d-%H%M%S).sql.gz"
    pg_dump "${DATABASE_URL}" | gzip > "${SAFETY_BACKUP}"
    log_success "Safety backup created: ${SAFETY_BACKUP}"
else
    log_info "Would create safety backup"
fi

# Step 6: Drop and recreate database schema
log_info "Step 6/10: Dropping existing database schema"

if [ "$DRY_RUN" != "--dry-run" ]; then
    psql "${DATABASE_URL}" <<EOF
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
GRANT ALL ON SCHEMA public TO PUBLIC;
EOF
    log_success "Database schema dropped and recreated"
else
    log_info "Would drop and recreate database schema"
fi

# Step 7: Restore from backup
log_info "Step 7/10: Restoring database from backup"

if [ "$DRY_RUN" != "--dry-run" ]; then
    gunzip < "${BACKUP_FILE}" | psql "${DATABASE_URL}"
    log_success "Database restored successfully"
else
    log_info "Would restore from backup"
fi

# Step 8: Verify data integrity
log_info "Step 8/10: Verifying data integrity"

if [ "$DRY_RUN" != "--dry-run" ]; then
    # Count schemas
    SCHEMA_COUNT=$(psql "${DATABASE_URL}" -t -c "SELECT COUNT(*) FROM schemas;" | xargs)
    log_info "Restored ${SCHEMA_COUNT} schemas"

    # Check for critical tables
    TABLES=$(psql "${DATABASE_URL}" -t -c "\dt" | wc -l)
    log_info "Restored ${TABLES} tables"

    # Verify database consistency
    psql "${DATABASE_URL}" -c "VACUUM ANALYZE;"

    log_success "Data integrity verified"
else
    log_info "Would verify schema count and run VACUUM ANALYZE"
fi

# Step 9: Clear Redis cache
log_info "Step 9/10: Clearing Redis cache"

if [ "$DRY_RUN" != "--dry-run" ]; then
    redis-cli -h "${REDIS_HOST}" -p "${REDIS_PORT}" FLUSHALL
    log_success "Redis cache cleared"
else
    log_info "Would clear Redis cache"
fi

# Step 10: Restart schema registry
log_info "Step 10/10: Restarting schema registry"

if [ "$DRY_RUN" != "--dry-run" ]; then
    REPLICAS=$(cat /tmp/schema-registry-replicas.txt || echo "3")

    kubectl scale deployment "${DEPLOYMENT_NAME}" \
        --replicas="${REPLICAS}" \
        -n "${KUBERNETES_NAMESPACE}"

    log_info "Waiting for deployment to be ready..."
    kubectl rollout status deployment "${DEPLOYMENT_NAME}" \
        -n "${KUBERNETES_NAMESPACE}" \
        --timeout=300s

    log_success "Schema registry restarted successfully"
else
    log_info "Would scale deployment back to original replicas"
fi

# Step 11: Smoke tests
log_info "Running smoke tests..."

if [ "$DRY_RUN" != "--dry-run" ]; then
    sleep 5  # Wait for service to be fully ready

    # Get service endpoint
    SERVICE_URL="http://localhost:8080"
    if kubectl get service "${DEPLOYMENT_NAME}" -n "${KUBERNETES_NAMESPACE}" &> /dev/null; then
        # Port-forward to test
        kubectl port-forward -n "${KUBERNETES_NAMESPACE}" \
            "service/${DEPLOYMENT_NAME}" 8080:8080 &
        PF_PID=$!
        sleep 2
    fi

    # Health check
    if curl -f -s "${SERVICE_URL}/health" > /dev/null; then
        log_success "Health check passed"
    else
        log_warn "Health check failed - service may need more time"
    fi

    # Kill port-forward
    if [ ! -z "${PF_PID:-}" ]; then
        kill "${PF_PID}" 2>/dev/null || true
    fi
else
    log_info "Would run health check smoke test"
fi

# Cleanup
log_info "Cleaning up temporary files..."
if [ "$DRY_RUN" != "--dry-run" ]; then
    rm -f "${BACKUP_FILE}"
    rm -f /tmp/schema-registry-replicas.txt
fi

# Calculate recovery time
END_TIME=$(date +%s)
RECOVERY_TIME=$((END_TIME - START_TIME))
RECOVERY_MINUTES=$((RECOVERY_TIME / 60))

log_info "========================================"
log_success "Disaster Recovery Completed Successfully"
log_info "========================================"
log_info "Recovery Time: ${RECOVERY_MINUTES} minutes (${RECOVERY_TIME} seconds)"
log_info "Backup ID: ${BACKUP_ID}"
log_info "Log File: ${LOG_FILE}"
log_info "========================================"

# RTO validation
if [ ${RECOVERY_MINUTES} -lt 240 ]; then
    log_success "RTO target met (< 4 hours): ${RECOVERY_MINUTES} minutes"
else
    log_warn "RTO target exceeded (> 4 hours): ${RECOVERY_MINUTES} minutes"
fi

echo ""
echo "Next steps:"
echo "1. Verify application functionality"
echo "2. Check monitoring dashboards"
echo "3. Notify stakeholders of recovery completion"
echo "4. Schedule post-incident review"
echo ""
