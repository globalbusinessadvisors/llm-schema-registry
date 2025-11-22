# Kubernetes Deployment Guide

Comprehensive guide for deploying LLM Schema Registry on Kubernetes.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Prerequisites](#prerequisites)
- [Deployment Options](#deployment-options)
- [Configuration](#configuration)
- [Networking](#networking)
- [Storage](#storage)
- [Security](#security)
- [Scaling](#scaling)
- [Operations](#operations)

---

## Architecture Overview

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Ingress                              │
│              (nginx/traefik + cert-manager)                  │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        v                v                v
┌───────────────┐ ┌──────────────┐ ┌──────────────┐
│ LoadBalancer  │ │  Service     │ │  Service     │
│   (HTTP)      │ │  (gRPC)      │ │  (Metrics)   │
└───────┬───────┘ └──────┬───────┘ └──────┬───────┘
        │                │                │
        └────────────────┼────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                 │
        v                                 v
┌───────────────┐                 ┌───────────────┐
│  Deployment   │                 │  Deployment   │
│  (3 replicas) │    <─HPA──>    │  (auto-scale) │
└───────┬───────┘                 └───────┬───────┘
        │                                 │
        ├─────────────┬───────────────────┤
        │             │                   │
        v             v                   v
┌──────────┐   ┌──────────┐       ┌──────────┐
│PostgreSQL│   │  Redis   │       │    S3    │
│StatefulSet   │StatefulSet       │ (External)│
└──────────┘   └──────────┘       └──────────┘
```

### Resource Distribution

- **Namespace:** schema-registry
- **Deployment:** 3+ replicas (configurable)
- **StatefulSets:** PostgreSQL, Redis
- **Services:** LoadBalancer, ClusterIP, Headless
- **ConfigMaps:** Application config
- **Secrets:** Credentials, JWT keys
- **PVCs:** Database persistence

---

## Prerequisites

### Cluster Requirements

- **Kubernetes Version:** 1.28+
- **Nodes:** Minimum 3 worker nodes
- **Node Resources:** 4 CPU, 8GB RAM per node
- **Storage Class:** Dynamic provisioning support
- **Container Runtime:** containerd, CRI-O, or Docker

### Required Add-ons

1. **Ingress Controller**
   ```bash
   # Nginx Ingress
   helm install ingress-nginx ingress-nginx/ingress-nginx \
     --namespace ingress-nginx \
     --create-namespace
   ```

2. **Cert-Manager** (for TLS)
   ```bash
   kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml
   ```

3. **Metrics Server** (for HPA)
   ```bash
   kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml
   ```

4. **Storage Provisioner**
   - AWS EBS CSI Driver
   - GCP Persistent Disk CSI Driver
   - Azure Disk CSI Driver
   - Or local-path-provisioner for development

---

## Deployment Options

### Option 1: Helm Chart (Recommended)

**Production deployment with all features:**

```bash
# Create namespace
kubectl create namespace schema-registry

# Install with Helm
helm install schema-registry ./helm/schema-registry \
  --namespace schema-registry \
  --create-namespace \
  --values - <<EOF
replicaCount: 3

image:
  registry: ghcr.io
  repository: llm-schema-registry/schema-registry
  tag: "0.1.0"

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: schema-registry.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: schema-registry-tls
      hosts:
        - schema-registry.example.com

postgresql:
  enabled: true
  primary:
    persistence:
      size: 50Gi
      storageClass: standard

redis:
  enabled: true
  master:
    persistence:
      size: 10Gi

s3:
  region: us-east-1
  bucket: schema-registry-artifacts
  accessKeyId: YOUR_KEY
  secretAccessKey: YOUR_SECRET

security:
  jwtSecret: YOUR_JWT_SECRET
EOF
```

### Option 2: Kustomize

**For GitOps workflows:**

```bash
# Base deployment
kubectl apply -k deployments/kubernetes/base/

# Environment-specific overlays
kubectl apply -k deployments/kubernetes/overlays/prod/
```

### Option 3: Raw Manifests

**For full control:**

```bash
# Apply in order
kubectl apply -f deployments/kubernetes/base/namespace.yaml
kubectl apply -f deployments/kubernetes/base/secrets.yaml
kubectl apply -f deployments/kubernetes/base/configmap.yaml
kubectl apply -f deployments/kubernetes/base/postgres-statefulset.yaml
kubectl apply -f deployments/kubernetes/base/redis-statefulset.yaml
kubectl apply -f deployments/kubernetes/base/serviceaccount.yaml
kubectl apply -f deployments/kubernetes/base/deployment.yaml
kubectl apply -f deployments/kubernetes/base/service.yaml
kubectl apply -f deployments/kubernetes/base/ingress.yaml
kubectl apply -f deployments/kubernetes/base/hpa.yaml
kubectl apply -f deployments/kubernetes/base/pdb.yaml
kubectl apply -f deployments/kubernetes/base/networkpolicy.yaml
```

---

## Configuration

### ConfigMap Structure

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: schema-registry-config
  namespace: schema-registry
data:
  # Server
  SERVER_HOST: "0.0.0.0"
  SERVER_PORT: "8080"
  GRPC_PORT: "9090"
  METRICS_PORT: "9091"

  # Database
  DATABASE_MAX_CONNECTIONS: "50"
  DATABASE_MIN_CONNECTIONS: "10"

  # Cache
  CACHE_TTL: "3600"
  CACHE_MAX_SIZE: "50000"

  # Logging
  RUST_LOG: "info,schema_registry=debug"
  LOG_FORMAT: "json"
```

### Secret Management

#### Option 1: Kubernetes Secrets

```bash
# Create secret manually
kubectl create secret generic schema-registry-secrets \
  --from-literal=DATABASE_URL=postgresql://user:pass@postgres:5432/db \
  --from-literal=REDIS_URL=redis://redis:6379 \
  --from-literal=JWT_SECRET=$(openssl rand -base64 32) \
  --from-literal=S3_ACCESS_KEY_ID=your-key \
  --from-literal=S3_SECRET_ACCESS_KEY=your-secret \
  --namespace schema-registry
```

#### Option 2: External Secrets Operator

```yaml
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
    creationPolicy: Owner
  data:
    - secretKey: DATABASE_URL
      remoteRef:
        key: prod/schema-registry/database-url
    - secretKey: JWT_SECRET
      remoteRef:
        key: prod/schema-registry/jwt-secret
```

#### Option 3: Sealed Secrets

```bash
# Install sealed-secrets controller
helm install sealed-secrets sealed-secrets/sealed-secrets \
  --namespace kube-system

# Create sealed secret
kubectl create secret generic schema-registry-secrets \
  --from-literal=JWT_SECRET=your-secret \
  --dry-run=client -o yaml | \
  kubeseal -o yaml > sealed-secret.yaml

# Apply sealed secret
kubectl apply -f sealed-secret.yaml
```

---

## Networking

### Service Types

**1. LoadBalancer (External Access)**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: schema-registry-service
spec:
  type: LoadBalancer
  selector:
    app.kubernetes.io/name: schema-registry
  ports:
    - name: http
      port: 80
      targetPort: 8080
    - name: grpc
      port: 9090
      targetPort: 9090
```

**2. ClusterIP (Internal Only)**
```yaml
spec:
  type: ClusterIP
  clusterIP: None  # Headless for StatefulSets
```

**3. NodePort (Development)**
```yaml
spec:
  type: NodePort
  ports:
    - port: 8080
      nodePort: 30080
```

### Ingress Configuration

**HTTP Ingress:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: schema-registry-ingress
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - schema-registry.example.com
      secretName: schema-registry-tls
  rules:
    - host: schema-registry.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: schema-registry-service
                port:
                  number: 80
```

**gRPC Ingress:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: schema-registry-grpc-ingress
  annotations:
    nginx.ingress.kubernetes.io/backend-protocol: "GRPC"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - grpc.schema-registry.example.com
      secretName: schema-registry-grpc-tls
  rules:
    - host: grpc.schema-registry.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: schema-registry-service
                port:
                  number: 9090
```

### Network Policies

**Default Policy (Restrictive):**
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: schema-registry-network-policy
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: schema-registry
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # Allow from ingress controller
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
        - protocol: TCP
          port: 9090
  egress:
    # Allow DNS
    - to:
        - namespaceSelector:
            matchLabels:
              name: kube-system
      ports:
        - protocol: UDP
          port: 53
    # Allow PostgreSQL
    - to:
        - podSelector:
            matchLabels:
              app: postgres
      ports:
        - protocol: TCP
          port: 5432
```

---

## Storage

### Storage Classes

**AWS EBS:**
```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: ebs.csi.aws.com
parameters:
  type: gp3
  iops: "3000"
  throughput: "125"
  encrypted: "true"
volumeBindingMode: WaitForFirstConsumer
allowVolumeExpansion: true
```

**GCP Persistent Disk:**
```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: pd.csi.storage.gke.io
parameters:
  type: pd-ssd
  replication-type: regional-pd
volumeBindingMode: WaitForFirstConsumer
allowVolumeExpansion: true
```

### Persistent Volume Claims

**PostgreSQL PVC:**
```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: postgres-data-postgres-0
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: fast-ssd
  resources:
    requests:
      storage: 100Gi
```

### Backup Strategy

**1. Volume Snapshots:**
```bash
# Create VolumeSnapshot
kubectl apply -f - <<EOF
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: postgres-snapshot-$(date +%Y%m%d)
  namespace: schema-registry
spec:
  volumeSnapshotClassName: csi-snapclass
  source:
    persistentVolumeClaimName: postgres-data-postgres-0
EOF
```

**2. Velero Backups:**
```bash
# Install Velero
velero install \
  --provider aws \
  --bucket velero-backups \
  --secret-file ./credentials-velero

# Create backup schedule
velero schedule create schema-registry-daily \
  --schedule="0 2 * * *" \
  --include-namespaces schema-registry \
  --ttl 720h0m0s

# Manual backup
velero backup create schema-registry-$(date +%Y%m%d) \
  --include-namespaces schema-registry
```

---

## Security

### Pod Security Standards

**Pod Security Context:**
```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  runAsGroup: 1000
  fsGroup: 1000
  seccompProfile:
    type: RuntimeDefault
```

**Container Security Context:**
```yaml
securityContext:
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  runAsNonRoot: true
  runAsUser: 1000
  capabilities:
    drop:
      - ALL
```

### RBAC

**Service Account:**
```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: schema-registry
  namespace: schema-registry
```

**Role:**
```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: schema-registry-role
rules:
  - apiGroups: [""]
    resources: ["configmaps", "secrets"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list"]
```

**RoleBinding:**
```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: schema-registry-rolebinding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: schema-registry-role
subjects:
  - kind: ServiceAccount
    name: schema-registry
```

---

## Scaling

### Horizontal Pod Autoscaler

**CPU/Memory-based:**
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: schema-registry-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: schema-registry
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

**Custom Metrics:**
```yaml
metrics:
  - type: Pods
    pods:
      metric:
        name: http_requests_per_second
      target:
        type: AverageValue
        averageValue: "1000"
```

### Vertical Pod Autoscaler

```yaml
apiVersion: autoscaling.k8s.io/v1
kind: VerticalPodAutoscaler
metadata:
  name: schema-registry-vpa
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: schema-registry
  updatePolicy:
    updateMode: "Auto"
  resourcePolicy:
    containerPolicies:
      - containerName: schema-registry
        minAllowed:
          cpu: 500m
          memory: 512Mi
        maxAllowed:
          cpu: 4000m
          memory: 8Gi
```

---

## Operations

### Health Checks

```bash
# Liveness probe
kubectl exec -it deployment/schema-registry -- \
  curl -f http://localhost:8080/health

# Readiness probe
kubectl exec -it deployment/schema-registry -- \
  curl -f http://localhost:8080/ready
```

### Rolling Updates

```bash
# Update image
kubectl set image deployment/schema-registry \
  schema-registry=ghcr.io/llm-schema-registry/schema-registry:0.2.0 \
  --namespace schema-registry

# Monitor rollout
kubectl rollout status deployment/schema-registry -n schema-registry

# Rollback if needed
kubectl rollout undo deployment/schema-registry -n schema-registry
```

### Debugging

```bash
# View logs
kubectl logs -f deployment/schema-registry -n schema-registry

# Execute commands in pod
kubectl exec -it deployment/schema-registry -n schema-registry -- /bin/sh

# Debug with ephemeral container
kubectl debug -it deployment/schema-registry --image=busybox -n schema-registry

# Port forward for local testing
kubectl port-forward svc/schema-registry-service 8080:80 -n schema-registry
```

### Monitoring

```bash
# Resource usage
kubectl top pods -n schema-registry
kubectl top nodes

# Events
kubectl get events -n schema-registry --sort-by='.lastTimestamp'

# Describe resources
kubectl describe deployment/schema-registry -n schema-registry
```

---

## Multi-Region Deployment

### Active-Active Setup

```yaml
# Region 1: us-east-1
apiVersion: v1
kind: Service
metadata:
  name: schema-registry-service
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
    external-dns.alpha.kubernetes.io/hostname: us-east-1.schema-registry.example.com

# Region 2: us-west-2
apiVersion: v1
kind: Service
metadata:
  name: schema-registry-service
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
    external-dns.alpha.kubernetes.io/hostname: us-west-2.schema-registry.example.com
```

### Database Replication

```yaml
# PostgreSQL with streaming replication
postgresql:
  architecture: replication
  replication:
    enabled: true
    user: replicator
    password: repl-password
    numSynchronousReplicas: 1
  readReplicas:
    replicaCount: 2
```

---

**Last Updated:** 2024-11-22
