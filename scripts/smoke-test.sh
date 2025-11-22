#!/bin/bash
# Smoke Test Suite for LLM Schema Registry
# Quick validation after deployment or recovery

set -euo pipefail

# Configuration
SERVICE_URL="${1:-http://localhost:8080}"
TIMEOUT=10

echo "=========================================="
echo "Schema Registry Smoke Tests"
echo "=========================================="
echo "Service URL: ${SERVICE_URL}"
echo "Timeout: ${TIMEOUT}s"
echo ""

PASSED=0
FAILED=0

# Test function
test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local expected_status="$4"
    local data="${5:-}"

    echo -n "Testing: ${name}... "

    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "${method}" \
            --connect-timeout "${TIMEOUT}" \
            "${SERVICE_URL}${endpoint}" || echo "000")
    else
        response=$(curl -s -w "\n%{http_code}" -X "${method}" \
            --connect-timeout "${TIMEOUT}" \
            -H "Content-Type: application/json" \
            -d "${data}" \
            "${SERVICE_URL}${endpoint}" || echo "000")
    fi

    status=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)

    if [ "$status" = "$expected_status" ]; then
        echo "✓ PASS (${status})"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo "✗ FAIL (expected ${expected_status}, got ${status})"
        echo "  Response: ${body}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Run tests
echo "1. Health Checks"
echo "----------------"
test_endpoint "Liveness Probe" "GET" "/health/live" "200"
test_endpoint "Readiness Probe" "GET" "/health/ready" "200"
test_endpoint "Startup Probe" "GET" "/health/startup" "200"
echo ""

echo "2. Metrics & Observability"
echo "--------------------------"
test_endpoint "Metrics Endpoint" "GET" "/metrics" "200"
echo ""

echo "3. API Endpoints"
echo "----------------"
test_endpoint "List Schemas (empty)" "GET" "/api/v1/schemas" "200"
test_endpoint "Invalid Endpoint" "GET" "/api/v1/invalid" "404"
echo ""

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "Passed: ${PASSED}"
echo "Failed: ${FAILED}"
echo "Total:  $((PASSED + FAILED))"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "✓ All smoke tests passed!"
    exit 0
else
    echo "✗ Some smoke tests failed!"
    exit 1
fi
