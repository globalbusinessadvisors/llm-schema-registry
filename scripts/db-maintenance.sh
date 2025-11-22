#!/bin/bash
# Database Maintenance Script for LLM Schema Registry
# Purpose: Perform routine PostgreSQL maintenance tasks
# Usage: ./db-maintenance.sh [--vacuum|--analyze|--reindex|--all]

set -euo pipefail

# Configuration
DATABASE_URL="${DATABASE_URL:-postgresql://postgres:postgres@localhost:5432/schema_registry}"
LOG_FILE="/var/log/schema-registry/db-maintenance-$(date +%Y%m%d-%H%M%S).log"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $*${NC}" | tee -a "${LOG_FILE}"
}

log_success() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] SUCCESS: $*${NC}" | tee -a "${LOG_FILE}"
}

log_warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $*${NC}" | tee -a "${LOG_FILE}"
}

# Create log directory
mkdir -p "$(dirname "${LOG_FILE}")"

# Parse command line arguments
OPERATION="${1:---all}"

log_info "========================================"
log_info "Database Maintenance Started"
log_info "Operation: ${OPERATION}"
log_info "========================================"

# Function to run VACUUM
run_vacuum() {
    log_info "Running VACUUM..."

    # VACUUM ANALYZE for better performance
    psql "${DATABASE_URL}" <<EOF
-- Vacuum all tables
VACUUM (VERBOSE, ANALYZE) schemas;
VACUUM (VERBOSE, ANALYZE) compatibility_checks;
VACUUM (VERBOSE, ANALYZE) audit_logs;
VACUUM (VERBOSE, ANALYZE) schema_metadata;

-- Check for bloat
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) AS external_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
EOF

    log_success "VACUUM completed"
}

# Function to run ANALYZE
run_analyze() {
    log_info "Running ANALYZE to update statistics..."

    psql "${DATABASE_URL}" <<EOF
-- Update statistics for query planner
ANALYZE VERBOSE schemas;
ANALYZE VERBOSE compatibility_checks;
ANALYZE VERBOSE audit_logs;
ANALYZE VERBOSE schema_metadata;

-- Show current statistics
SELECT
    schemaname,
    tablename,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze,
    n_tup_ins,
    n_tup_upd,
    n_tup_del
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY tablename;
EOF

    log_success "ANALYZE completed"
}

# Function to rebuild indexes
run_reindex() {
    log_info "Rebuilding indexes..."

    # List current indexes
    log_info "Current indexes:"
    psql "${DATABASE_URL}" -c "\di"

    # Reindex tables
    psql "${DATABASE_URL}" <<EOF
-- Reindex all tables (CONCURRENTLY to avoid locks)
REINDEX TABLE CONCURRENTLY schemas;
REINDEX TABLE CONCURRENTLY compatibility_checks;
REINDEX TABLE CONCURRENTLY audit_logs;
REINDEX TABLE CONCURRENTLY schema_metadata;

-- Show index sizes
SELECT
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) AS index_size
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY pg_relation_size(indexrelid) DESC;
EOF

    log_success "REINDEX completed"
}

# Function to check database bloat
check_bloat() {
    log_info "Checking for table bloat..."

    psql "${DATABASE_URL}" <<EOF
-- Check table bloat
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    round(100 * pg_relation_size(schemaname||'.'||tablename)::numeric /
          NULLIF(pg_total_relation_size(schemaname||'.'||tablename), 0), 2) AS table_pct
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
EOF

    log_success "Bloat check completed"
}

# Function to check long-running queries
check_queries() {
    log_info "Checking for long-running queries..."

    psql "${DATABASE_URL}" <<EOF
-- Long-running queries
SELECT
    pid,
    now() - pg_stat_activity.query_start AS duration,
    query,
    state
FROM pg_stat_activity
WHERE (now() - pg_stat_activity.query_start) > interval '1 minute'
  AND state != 'idle'
ORDER BY duration DESC;
EOF

    log_success "Query check completed"
}

# Function to show database statistics
show_stats() {
    log_info "Database statistics:"

    psql "${DATABASE_URL}" <<EOF
-- Database size
SELECT pg_size_pretty(pg_database_size(current_database())) AS database_size;

-- Table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) -
                  pg_relation_size(schemaname||'.'||tablename)) AS indexes_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Connection statistics
SELECT
    count(*) FILTER (WHERE state = 'active') AS active,
    count(*) FILTER (WHERE state = 'idle') AS idle,
    count(*) FILTER (WHERE state = 'idle in transaction') AS idle_in_transaction,
    count(*) AS total
FROM pg_stat_activity
WHERE datname = current_database();

-- Cache hit ratio
SELECT
    sum(heap_blks_read) as heap_read,
    sum(heap_blks_hit)  as heap_hit,
    sum(heap_blks_hit) / (sum(heap_blks_hit) + sum(heap_blks_read)) * 100 as cache_hit_ratio
FROM pg_statio_user_tables;
EOF

    log_success "Statistics retrieved"
}

# Execute operations based on command line argument
case "${OPERATION}" in
    --vacuum)
        run_vacuum
        ;;
    --analyze)
        run_analyze
        ;;
    --reindex)
        run_reindex
        ;;
    --bloat)
        check_bloat
        ;;
    --queries)
        check_queries
        ;;
    --stats)
        show_stats
        ;;
    --all)
        log_info "Running full maintenance..."
        show_stats
        check_queries
        check_bloat
        run_vacuum
        run_analyze
        # Skip reindex in --all to avoid long locks
        log_info "Skipping REINDEX in --all mode (run manually with --reindex)"
        ;;
    *)
        echo "Usage: $0 [--vacuum|--analyze|--reindex|--bloat|--queries|--stats|--all]"
        echo ""
        echo "Options:"
        echo "  --vacuum   Run VACUUM on all tables"
        echo "  --analyze  Update statistics for query planner"
        echo "  --reindex  Rebuild all indexes"
        echo "  --bloat    Check for table/index bloat"
        echo "  --queries  Show long-running queries"
        echo "  --stats    Show database statistics"
        echo "  --all      Run all maintenance tasks (default)"
        exit 1
        ;;
esac

log_info "========================================"
log_success "Database Maintenance Completed"
log_info "Log: ${LOG_FILE}"
log_info "========================================"
