#!/bin/bash
# Capacity Planning Script for LLM Schema Registry
# Purpose: Analyze resource usage and project future capacity needs
# Usage: ./capacity-planning.sh [--report|--forecast]

set -euo pipefail

# Configuration
PROMETHEUS_URL="${PROMETHEUS_URL:-http://localhost:9090}"
FORECAST_DAYS="${FORECAST_DAYS:-90}"
OUTPUT_DIR="/tmp/capacity-reports"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}"
}

log_success() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}"
}

log_warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}"
}

log_error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] $*${NC}"
}

# Create output directory
mkdir -p "${OUTPUT_DIR}"
REPORT_FILE="${OUTPUT_DIR}/capacity-report-$(date +%Y%m%d-%H%M%S).txt"

log_info "========================================"
log_info "Capacity Planning Analysis"
log_info "========================================"

# Function to query Prometheus
query_prometheus() {
    local query="$1"
    local result=$(curl -s -G "${PROMETHEUS_URL}/api/v1/query" \
        --data-urlencode "query=${query}" | \
        jq -r '.data.result[0].value[1]' 2>/dev/null || echo "N/A")
    echo "${result}"
}

# Function to analyze CPU usage
analyze_cpu() {
    log_info "Analyzing CPU usage..."

    echo "=== CPU Usage Analysis ===" >> "${REPORT_FILE}"

    # Current CPU usage
    local cpu_usage=$(query_prometheus 'rate(container_cpu_usage_seconds_total{pod=~"schema-registry.*"}[5m])')
    echo "Current CPU usage: ${cpu_usage} cores" >> "${REPORT_FILE}"

    # Peak CPU usage (last 7 days)
    local cpu_peak=$(query_prometheus 'max_over_time(rate(container_cpu_usage_seconds_total{pod=~"schema-registry.*"}[5m])[7d:5m])')
    echo "Peak CPU usage (7d): ${cpu_peak} cores" >> "${REPORT_FILE}"

    # Average CPU usage (last 30 days)
    local cpu_avg=$(query_prometheus 'avg_over_time(rate(container_cpu_usage_seconds_total{pod=~"schema-registry.*"}[5m])[30d:5m])')
    echo "Average CPU usage (30d): ${cpu_avg} cores" >> "${REPORT_FILE}"

    # CPU limit
    local cpu_limit=$(query_prometheus 'kube_pod_container_resource_limits{pod=~"schema-registry.*",resource="cpu"}')
    echo "CPU limit: ${cpu_limit} cores" >> "${REPORT_FILE}"

    # Calculate utilization
    if [ "${cpu_usage}" != "N/A" ] && [ "${cpu_limit}" != "N/A" ]; then
        local cpu_pct=$(awk "BEGIN {printf \"%.2f\", (${cpu_usage}/${cpu_limit})*100}")
        echo "CPU utilization: ${cpu_pct}%" >> "${REPORT_FILE}"

        if (( $(echo "${cpu_pct} > 80" | bc -l) )); then
            log_warn "CPU utilization > 80%: Consider scaling up"
            echo "WARNING: High CPU utilization" >> "${REPORT_FILE}"
        fi
    fi

    echo "" >> "${REPORT_FILE}"
    log_success "CPU analysis complete"
}

# Function to analyze memory usage
analyze_memory() {
    log_info "Analyzing memory usage..."

    echo "=== Memory Usage Analysis ===" >> "${REPORT_FILE}"

    # Current memory usage
    local mem_usage=$(query_prometheus 'container_memory_usage_bytes{pod=~"schema-registry.*"}')
    local mem_usage_mb=$(awk "BEGIN {printf \"%.2f\", ${mem_usage:-0}/1024/1024}")
    echo "Current memory usage: ${mem_usage_mb} MB" >> "${REPORT_FILE}"

    # Peak memory usage (last 7 days)
    local mem_peak=$(query_prometheus 'max_over_time(container_memory_usage_bytes{pod=~"schema-registry.*"}[7d])')
    local mem_peak_mb=$(awk "BEGIN {printf \"%.2f\", ${mem_peak:-0}/1024/1024}")
    echo "Peak memory usage (7d): ${mem_peak_mb} MB" >> "${REPORT_FILE}"

    # Memory limit
    local mem_limit=$(query_prometheus 'kube_pod_container_resource_limits{pod=~"schema-registry.*",resource="memory"}')
    local mem_limit_mb=$(awk "BEGIN {printf \"%.2f\", ${mem_limit:-0}/1024/1024}")
    echo "Memory limit: ${mem_limit_mb} MB" >> "${REPORT_FILE}"

    # Calculate utilization
    if [ "${mem_usage}" != "N/A" ] && [ "${mem_limit}" != "N/A" ]; then
        local mem_pct=$(awk "BEGIN {printf \"%.2f\", (${mem_usage}/${mem_limit})*100}")
        echo "Memory utilization: ${mem_pct}%" >> "${REPORT_FILE}"

        if (( $(echo "${mem_pct} > 80" | bc -l) )); then
            log_warn "Memory utilization > 80%: Consider scaling up"
            echo "WARNING: High memory utilization" >> "${REPORT_FILE}"
        fi
    fi

    echo "" >> "${REPORT_FILE}"
    log_success "Memory analysis complete"
}

# Function to analyze request rate
analyze_requests() {
    log_info "Analyzing request rate..."

    echo "=== Request Rate Analysis ===" >> "${REPORT_FILE}"

    # Current request rate
    local req_rate=$(query_prometheus 'rate(schema_registry_http_requests_total[5m])')
    echo "Current request rate: ${req_rate} req/s" >> "${REPORT_FILE}"

    # Peak request rate (last 7 days)
    local req_peak=$(query_prometheus 'max_over_time(rate(schema_registry_http_requests_total[5m])[7d:5m])')
    echo "Peak request rate (7d): ${req_peak} req/s" >> "${REPORT_FILE}"

    # Average request rate (last 30 days)
    local req_avg=$(query_prometheus 'avg_over_time(rate(schema_registry_http_requests_total[5m])[30d:5m])')
    echo "Average request rate (30d): ${req_avg} req/s" >> "${REPORT_FILE}"

    # Growth rate (comparing last 7 days vs previous 7 days)
    local req_current=$(query_prometheus 'avg_over_time(rate(schema_registry_http_requests_total[5m])[7d:5m])')
    local req_previous=$(query_prometheus 'avg_over_time(rate(schema_registry_http_requests_total[5m])[14d:5m] offset 7d)')

    if [ "${req_current}" != "N/A" ] && [ "${req_previous}" != "N/A" ] && [ "${req_previous}" != "0" ]; then
        local growth=$(awk "BEGIN {printf \"%.2f\", ((${req_current}-${req_previous})/${req_previous})*100}")
        echo "Weekly growth rate: ${growth}%" >> "${REPORT_FILE}"
    fi

    echo "" >> "${REPORT_FILE}"
    log_success "Request rate analysis complete"
}

# Function to analyze database usage
analyze_database() {
    log_info "Analyzing database usage..."

    echo "=== Database Usage Analysis ===" >> "${REPORT_FILE}"

    # Database connections
    local db_connections=$(query_prometheus 'schema_registry_db_connections_active{pool="postgres"}')
    echo "Active DB connections: ${db_connections}" >> "${REPORT_FILE}"

    # Connection pool utilization
    local pool_max=$(query_prometheus 'schema_registry_db_connections_max{pool="postgres"}')
    if [ "${db_connections}" != "N/A" ] && [ "${pool_max}" != "N/A" ]; then
        local pool_pct=$(awk "BEGIN {printf \"%.2f\", (${db_connections}/${pool_max})*100}")
        echo "Connection pool utilization: ${pool_pct}%" >> "${REPORT_FILE}"

        if (( $(echo "${pool_pct} > 80" | bc -l) )); then
            log_warn "Connection pool utilization > 80%: Consider increasing pool size"
            echo "WARNING: High connection pool utilization" >> "${REPORT_FILE}"
        fi
    fi

    # Query latency
    local query_latency=$(query_prometheus 'histogram_quantile(0.95, rate(schema_registry_db_query_duration_seconds_bucket[5m]))')
    echo "Query latency (p95): ${query_latency}s" >> "${REPORT_FILE}"

    echo "" >> "${REPORT_FILE}"
    log_success "Database analysis complete"
}

# Function to analyze cache usage
analyze_cache() {
    log_info "Analyzing cache usage..."

    echo "=== Cache Usage Analysis ===" >> "${REPORT_FILE}"

    # Cache hit rate
    local cache_hit_rate=$(query_prometheus 'schema_registry_cache_hit_rate{tier="L1"}')
    echo "Cache hit rate (L1): ${cache_hit_rate}" >> "${REPORT_FILE}"

    if [ "${cache_hit_rate}" != "N/A" ]; then
        local hit_pct=$(awk "BEGIN {printf \"%.2f\", ${cache_hit_rate}*100}")
        echo "Cache hit percentage: ${hit_pct}%" >> "${REPORT_FILE}"

        if (( $(echo "${hit_pct} < 95" | bc -l) )); then
            log_warn "Cache hit rate < 95%: Consider cache tuning"
            echo "WARNING: Low cache hit rate" >> "${REPORT_FILE}"
        fi
    fi

    # Redis memory usage
    local redis_mem=$(query_prometheus 'redis_memory_used_bytes')
    local redis_mem_mb=$(awk "BEGIN {printf \"%.2f\", ${redis_mem:-0}/1024/1024}")
    echo "Redis memory usage: ${redis_mem_mb} MB" >> "${REPORT_FILE}"

    echo "" >> "${REPORT_FILE}"
    log_success "Cache analysis complete"
}

# Function to generate forecast
generate_forecast() {
    log_info "Generating ${FORECAST_DAYS}-day forecast..."

    echo "=== ${FORECAST_DAYS}-Day Capacity Forecast ===" >> "${REPORT_FILE}"

    # Get current metrics and growth rates
    local current_req=$(query_prometheus 'rate(schema_registry_http_requests_total[5m])')
    local growth_rate="0.05"  # Assume 5% weekly growth if can't calculate

    # Calculate projected metrics
    local weeks=$((FORECAST_DAYS / 7))
    local multiplier=$(awk "BEGIN {printf \"%.4f\", (1 + ${growth_rate}) ^ ${weeks}}")

    if [ "${current_req}" != "N/A" ]; then
        local projected_req=$(awk "BEGIN {printf \"%.2f\", ${current_req} * ${multiplier}}")
        echo "Projected request rate: ${projected_req} req/s" >> "${REPORT_FILE}"

        # Calculate required replicas (assume 2000 req/s per replica)
        local required_replicas=$(awk "BEGIN {printf \"%.0f\", ${projected_req} / 2000 + 0.5}")
        [ "${required_replicas}" -lt "3" ] && required_replicas="3"  # Minimum 3 for HA
        echo "Recommended replicas: ${required_replicas}" >> "${REPORT_FILE}"
    fi

    echo "" >> "${REPORT_FILE}"

    echo "=== Resource Recommendations ===" >> "${REPORT_FILE}"
    echo "1. Monitor CPU/memory usage weekly" >> "${REPORT_FILE}"
    echo "2. Scale horizontally when avg utilization > 60%" >> "${REPORT_FILE}"
    echo "3. Plan capacity expansion at 80% utilization" >> "${REPORT_FILE}"
    echo "4. Review database connection pool size monthly" >> "${REPORT_FILE}"
    echo "5. Monitor cache hit rates and tune as needed" >> "${REPORT_FILE}"

    echo "" >> "${REPORT_FILE}"
    log_success "Forecast generated"
}

# Function to generate cost estimate
estimate_costs() {
    log_info "Generating cost estimates..."

    echo "=== Cost Estimates ===" >> "${REPORT_FILE}"

    # Assumptions (modify based on your cloud provider)
    local cost_per_cpu_hour="0.04"  # $0.04 per vCPU hour
    local cost_per_gb_hour="0.005"  # $0.005 per GB hour

    # Current resources
    local cpu_cores=$(query_prometheus 'sum(kube_pod_container_resource_requests{pod=~"schema-registry.*",resource="cpu"})')
    local mem_gb=$(query_prometheus 'sum(kube_pod_container_resource_requests{pod=~"schema-registry.*",resource="memory"}) / 1024 / 1024 / 1024')

    # Monthly costs
    local monthly_cpu_cost=$(awk "BEGIN {printf \"%.2f\", ${cpu_cores:-0} * ${cost_per_cpu_hour} * 24 * 30}")
    local monthly_mem_cost=$(awk "BEGIN {printf \"%.2f\", ${mem_gb:-0} * ${cost_per_gb_hour} * 24 * 30}")
    local monthly_total=$(awk "BEGIN {printf \"%.2f\", ${monthly_cpu_cost} + ${monthly_mem_cost}}")

    echo "Monthly compute cost: \$${monthly_total}" >> "${REPORT_FILE}"
    echo "  - CPU: \$${monthly_cpu_cost}" >> "${REPORT_FILE}"
    echo "  - Memory: \$${monthly_mem_cost}" >> "${REPORT_FILE}"

    echo "" >> "${REPORT_FILE}"
    log_success "Cost estimates generated"
}

# Main execution
log_info "Starting capacity planning analysis..."

analyze_cpu
analyze_memory
analyze_requests
analyze_database
analyze_cache
generate_forecast
estimate_costs

# Generate summary
echo "=== Summary ===" >> "${REPORT_FILE}"
echo "Report generated: $(date)" >> "${REPORT_FILE}"
echo "Prometheus URL: ${PROMETHEUS_URL}" >> "${REPORT_FILE}"
echo "Forecast period: ${FORECAST_DAYS} days" >> "${REPORT_FILE}"

log_info "========================================"
log_success "Capacity Planning Analysis Complete"
log_info "Report saved to: ${REPORT_FILE}"
log_info "========================================"

# Display report
cat "${REPORT_FILE}"
