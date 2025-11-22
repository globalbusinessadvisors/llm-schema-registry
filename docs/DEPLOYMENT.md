# Deployment Guide

Complete deployment guide for LLM Schema Registry across different environments and platforms.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Deployment](#docker-deployment)
- [Docker Compose](#docker-compose)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Production Considerations](#production-considerations)
- [Monitoring & Observability](#monitoring--observability)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

- **CPU:** Minimum 2 cores, Recommended 4+ cores
- **RAM:** Minimum 4GB, Recommended 8GB+
- **Storage:** Minimum 50GB SSD

### Software Requirements

- Docker 24.0+
- Docker Compose 2.20+
- Kubernetes 1.28+ (for K8s deployment)
- Helm 3.13+ (for Helm deployment)
- kubectl (for K8s management)

### External Dependencies

- **PostgreSQL 14+:** Primary metadata storage
- **Redis 7+:** Caching layer
- **S3-compatible storage:** Schema artifact storage (AWS S3, MinIO, LocalStack)

---

## Docker Deployment

### Quick Start with Docker

Build and run the Schema Registry server using Docker:

```bash
# Build the Docker image
docker build -t schema-registry:latest .

# Run with minimal configuration
docker run -d \
  --name schema-registry \
  -p 8080:8080 \
  -p 9090:9090 \
  -p 9091:9091 \
  -e DATABASE_URL=postgresql://user:pass@postgres:5432/schema_registry \
  -e REDIS_URL=redis://redis:6379 \
  -e S3_ENDPOINT=http://s3:9000 \
  -e S3_REGION=us-east-1 \
  -e S3_BUCKET=schema-registry-artifacts \
  -e JWT_SECRET=your-secret-key \
  schema-registry:latest

# Check logs
docker logs -f schema-registry

# Health check
curl http://localhost:8080/health
```

### Docker Image Variants

**Server Image (Default):**
```bash
docker build --target runtime-server -t schema-registry:server .
```

**CLI Image:**
```bash
docker build --target runtime-cli -t schema-registry:cli .
docker run --rm schema-registry:cli --help
```

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | - | PostgreSQL connection string |
| `REDIS_URL` | Yes | - | Redis connection string |
| `S3_ENDPOINT` | No | AWS S3 | S3-compatible endpoint |
| `S3_REGION` | Yes | us-east-1 | AWS region |
| `S3_BUCKET` | Yes | - | S3 bucket name |
| `S3_ACCESS_KEY_ID` | Yes | - | S3 access key |
| `S3_SECRET_ACCESS_KEY` | Yes | - | S3 secret key |
| `JWT_SECRET` | Yes | - | JWT signing secret |
| `SERVER_PORT` | No | 8080 | HTTP server port |
| `GRPC_PORT` | No | 9090 | gRPC server port |
| `METRICS_PORT` | No | 9091 | Metrics endpoint port |
| `RUST_LOG` | No | info | Logging level |

---

## Docker Compose

### Development Setup

Start the complete stack (PostgreSQL, Redis, LocalStack, Schema Registry):

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f schema-registry

# Check service status
docker-compose ps

# Run migrations (if needed)
docker-compose exec schema-registry /app/schema-registry-server migrate

# Stop all services
docker-compose down

# Remove volumes (careful: deletes all data!)
docker-compose down -v
```

### Production Setup with Monitoring

Enable monitoring stack (Prometheus + Grafana):

```bash
# Start with monitoring profile
docker-compose --profile monitoring up -d

# Access services:
# - Schema Registry: http://localhost:8080
# - Prometheus: http://localhost:9092
# - Grafana: http://localhost:3000 (admin/admin)
# - Metrics: http://localhost:9091/metrics
```

### Custom Configuration

Create a `.env` file for custom configuration:

```bash
# Copy example configuration
cp .env.example .env

# Edit configuration
nano .env

# Start with custom config
docker-compose --env-file .env up -d
```

Example `.env`:

```env
POSTGRES_PASSWORD=secure_password
JWT_SECRET=your-jwt-secret-key-here
S3_ACCESS_KEY_ID=your-access-key
S3_SECRET_ACCESS_KEY=your-secret-key
GRAFANA_PASSWORD=admin
```

---

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster (1.28+)
- kubectl configured
- Storage class available
- Ingress controller (nginx/traefik)
- cert-manager (for TLS)

### Quick Start with kubectl

Deploy using raw Kubernetes manifests:

```bash
# Create namespace
kubectl create namespace schema-registry

# Deploy PostgreSQL
kubectl apply -f deployments/kubernetes/base/postgres-statefulset.yaml

# Deploy Redis
kubectl apply -f deployments/kubernetes/base/redis-statefulset.yaml

# Wait for databases to be ready
kubectl wait --for=condition=ready pod -l app=postgres -n schema-registry --timeout=300s
kubectl wait --for=condition=ready pod -l app=redis -n schema-registry --timeout=300s

# Create secrets (update with your values)
kubectl create secret generic schema-registry-secrets \
  --from-literal=DATABASE_URL=postgresql://schema_registry:PASSWORD@postgres-service:5432/schema_registry \
  --from-literal=REDIS_URL=redis://redis-service:6379 \
  --from-literal=JWT_SECRET=your-jwt-secret \
  --from-literal=S3_ACCESS_KEY_ID=your-key \
  --from-literal=S3_SECRET_ACCESS_KEY=your-secret \
  -n schema-registry

# Deploy application
kubectl apply -f deployments/kubernetes/base/

# Check deployment status
kubectl get pods -n schema-registry
kubectl get svc -n schema-registry

# View logs
kubectl logs -f deployment/schema-registry -n schema-registry

# Port forward for testing
kubectl port-forward svc/schema-registry-service 8080:80 -n schema-registry
```

### Deployment with Kustomize

For environment-specific deployments:

```bash
# Development
kubectl apply -k deployments/kubernetes/overlays/dev/

# Staging
kubectl apply -k deployments/kubernetes/overlays/staging/

# Production
kubectl apply -k deployments/kubernetes/overlays/prod/
```

### Helm Deployment (Recommended)

#### Install from Local Chart

```bash
# Add dependencies (if using external charts)
helm dependency update helm/schema-registry

# Install
helm install schema-registry helm/schema-registry \
  --namespace schema-registry \
  --create-namespace \
  --values helm/schema-registry/values.yaml

# Check status
helm status schema-registry -n schema-registry
kubectl get pods -n schema-registry

# View installation notes
helm get notes schema-registry -n schema-registry
```

#### Install from OCI Registry (Future)

```bash
# Add repository
helm repo add schema-registry https://llm-schema-registry.github.io/charts
helm repo update

# Install
helm install schema-registry schema-registry/schema-registry \
  --namespace schema-registry \
  --create-namespace \
  --set image.tag=0.1.0
```

#### Custom Values

Create a `custom-values.yaml`:

```yaml
replicaCount: 5

resources:
  limits:
    cpu: 4000m
    memory: 4Gi
  requests:
    cpu: 1000m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 20
  targetCPUUtilizationPercentage: 60

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: schema-registry.your-domain.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: schema-registry-tls
      hosts:
        - schema-registry.your-domain.com

postgresql:
  enabled: true
  primary:
    persistence:
      size: 100Gi

redis:
  enabled: true
  master:
    persistence:
      size: 20Gi

s3:
  region: us-west-2
  bucket: prod-schema-registry-artifacts
```

Install with custom values:

```bash
helm install schema-registry helm/schema-registry \
  --namespace schema-registry \
  --create-namespace \
  --values custom-values.yaml
```

#### Upgrade

```bash
# Upgrade to new version
helm upgrade schema-registry helm/schema-registry \
  --namespace schema-registry \
  --values custom-values.yaml

# Rollback if needed
helm rollback schema-registry -n schema-registry
```

#### Uninstall

```bash
# Uninstall release
helm uninstall schema-registry -n schema-registry

# Delete namespace (optional, removes all data)
kubectl delete namespace schema-registry
```

---

## Production Considerations

### Security

1. **Secrets Management**
   - Use external secret managers (AWS Secrets Manager, HashiCorp Vault, etc.)
   - Enable Kubernetes secrets encryption at rest
   - Rotate secrets regularly

   ```bash
   # Example: Using External Secrets Operator
   kubectl apply -f - <<EOF
   apiVersion: external-secrets.io/v1beta1
   kind: ExternalSecret
   metadata:
     name: schema-registry-secrets
     namespace: schema-registry
   spec:
     refreshInterval: 1h
     secretStoreRef:
       name: aws-secrets-manager
       kind: SecretStore
     target:
       name: schema-registry-secrets
     data:
       - secretKey: DATABASE_URL
         remoteRef:
           key: prod/schema-registry/database-url
   EOF
   ```

2. **Network Policies**
   - Enabled by default in Helm chart
   - Restricts ingress/egress traffic
   - Review and customize for your environment

3. **TLS/SSL**
   - Enable TLS for all external endpoints
   - Use cert-manager for automatic certificate management

   ```bash
   # Install cert-manager
   kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

   # Create ClusterIssuer
   kubectl apply -f - <<EOF
   apiVersion: cert-manager.io/v1
   kind: ClusterIssuer
   metadata:
     name: letsencrypt-prod
   spec:
     acme:
       server: https://acme-v02.api.letsencrypt.org/directory
       email: admin@your-domain.com
       privateKeySecretRef:
         name: letsencrypt-prod
       solvers:
       - http01:
           ingress:
             class: nginx
   EOF
   ```

4. **RBAC**
   - Service account with minimal permissions
   - Pod security policies/standards enabled
   - Read-only root filesystem

### High Availability

1. **Multiple Replicas**
   ```yaml
   replicaCount: 3  # Minimum for HA
   ```

2. **Pod Disruption Budget**
   ```yaml
   podDisruptionBudget:
     enabled: true
     minAvailable: 2
   ```

3. **Anti-affinity Rules**
   - Spread pods across availability zones
   - Configured by default in Helm chart

4. **Database HA**
   ```yaml
   postgresql:
     architecture: replication
     replication:
       enabled: true
       numSynchronousReplicas: 1
   ```

### Scaling

1. **Horizontal Pod Autoscaling**
   - Enabled by default
   - Scales based on CPU, memory, and custom metrics

2. **Vertical Pod Autoscaling**
   ```bash
   # Install VPA
   kubectl apply -f https://github.com/kubernetes/autoscaler/releases/download/vertical-pod-autoscaler-0.14.0/vpa-v0.14.0.yaml

   # Create VPA for Schema Registry
   kubectl apply -f - <<EOF
   apiVersion: autoscaling.k8s.io/v1
   kind: VerticalPodAutoscaler
   metadata:
     name: schema-registry-vpa
     namespace: schema-registry
   spec:
     targetRef:
       apiVersion: apps/v1
       kind: Deployment
       name: schema-registry
     updatePolicy:
       updateMode: "Auto"
   EOF
   ```

### Backup & Disaster Recovery

1. **PostgreSQL Backups**
   ```bash
   # Manual backup
   kubectl exec -n schema-registry postgres-0 -- \
     pg_dump -U schema_registry schema_registry > backup.sql

   # Automated backups with Velero
   velero backup create schema-registry-backup \
     --include-namespaces schema-registry
   ```

2. **S3 Versioning**
   - Enable versioning on S3 bucket
   - Configure lifecycle policies

3. **Disaster Recovery Plan**
   - Document recovery procedures
   - Test regularly (quarterly)
   - Maintain off-site backups

### Performance Tuning

1. **Database Connection Pool**
   ```yaml
   externalDatabase:
     maxConnections: 100
     minConnections: 20
   ```

2. **Redis Caching**
   ```yaml
   config:
     cache:
       ttl: 7200
       maxSize: 100000
   ```

3. **Resource Requests/Limits**
   - Set based on actual usage
   - Monitor and adjust

---

## Monitoring & Observability

### Metrics

Schema Registry exposes Prometheus metrics on port 9091:

```bash
# Access metrics
curl http://localhost:9091/metrics
```

Key metrics:
- `schema_registry_requests_total` - Total HTTP requests
- `schema_registry_request_duration_seconds` - Request latency
- `schema_registry_schemas_total` - Total schemas registered
- `schema_registry_cache_hits_total` - Cache hit rate
- `schema_registry_database_connections` - Active DB connections

### Prometheus Setup

```yaml
# ServiceMonitor for Prometheus Operator
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: schema-registry
  namespace: schema-registry
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: schema-registry
  endpoints:
    - port: metrics
      interval: 30s
```

### Grafana Dashboards

Import the provided dashboards:

```bash
# Port forward to Grafana
kubectl port-forward -n monitoring svc/grafana 3000:3000

# Import dashboard from deployments/monitoring/grafana/dashboards/
```

### Logging

Logs are sent to stdout in JSON format:

```bash
# View logs
kubectl logs -f deployment/schema-registry -n schema-registry

# With stern (multi-pod)
stern schema-registry -n schema-registry

# Query with jq
kubectl logs deployment/schema-registry -n schema-registry | jq '.level, .msg'
```

### Distributed Tracing

Integration with OpenTelemetry/Jaeger:

```yaml
env:
  - name: OTEL_EXPORTER_OTLP_ENDPOINT
    value: "http://jaeger-collector:4317"
  - name: OTEL_SERVICE_NAME
    value: "schema-registry"
```

---

## Troubleshooting

### Common Issues

#### Pods Not Starting

```bash
# Check pod status
kubectl describe pod <pod-name> -n schema-registry

# Check logs
kubectl logs <pod-name> -n schema-registry

# Common causes:
# - Image pull errors: Check registry credentials
# - Init container failures: Check database connectivity
# - Resource constraints: Check node resources
```

#### Database Connection Errors

```bash
# Test database connectivity
kubectl run -it --rm psql --image=postgres:16 --restart=Never -- \
  psql postgresql://schema_registry:PASSWORD@postgres-service.schema-registry:5432/schema_registry

# Check PostgreSQL logs
kubectl logs -f postgres-0 -n schema-registry
```

#### High Latency

```bash
# Check resource usage
kubectl top pods -n schema-registry

# Check HPA status
kubectl get hpa -n schema-registry

# Review metrics
curl http://localhost:9091/metrics | grep duration
```

#### Cache Issues

```bash
# Connect to Redis
kubectl exec -it redis-0 -n schema-registry -- redis-cli

# Check cache stats
INFO stats

# Clear cache (if needed)
FLUSHDB
```

### Debug Mode

Enable debug logging:

```bash
helm upgrade schema-registry helm/schema-registry \
  --set config.logging.level="debug,schema_registry=trace" \
  --reuse-values
```

### Health Checks

```bash
# Liveness
curl http://localhost:8080/health

# Readiness
curl http://localhost:8080/ready

# Metrics
curl http://localhost:9091/metrics
```

---

## Migration Guide

### From Docker Compose to Kubernetes

1. **Export data from PostgreSQL**
   ```bash
   docker-compose exec postgres pg_dump -U schema_registry schema_registry > backup.sql
   ```

2. **Create K8s secrets with same credentials**

3. **Deploy to Kubernetes**

4. **Import data**
   ```bash
   kubectl exec -i postgres-0 -n schema-registry -- \
     psql -U schema_registry schema_registry < backup.sql
   ```

### Zero-Downtime Updates

```bash
# Helm upgrade with rolling update
helm upgrade schema-registry helm/schema-registry \
  --set image.tag=0.2.0 \
  --wait --timeout=10m

# Monitor rollout
kubectl rollout status deployment/schema-registry -n schema-registry
```

---

## Support

- **Documentation:** https://github.com/llm-schema-registry/llm-schema-registry
- **Issues:** https://github.com/llm-schema-registry/llm-schema-registry/issues
- **Discussions:** https://github.com/llm-schema-registry/llm-schema-registry/discussions

---

**Last Updated:** 2024-11-22
