# RAPS Helm Chart

Deploy RAPS distributed orchestration on Kubernetes.

## Prerequisites

- Kubernetes 1.27+
- Helm 3.12+
- (Optional) cert-manager for TLS
- (Optional) Prometheus Operator for ServiceMonitor
- (Optional) Sealed Secrets controller for encrypted credentials

## Quick Start

```bash
# Add Bitnami repo for Redis subchart
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo update

# Install with default values
helm install raps ./deploy/helm/raps/ \
  --namespace raps-system --create-namespace \
  --set secrets.apsCredentials.clientId=YOUR_CLIENT_ID \
  --set secrets.apsCredentials.clientSecret=YOUR_CLIENT_SECRET

# Check status
kubectl get pods -n raps-system
kubectl get hpa -n raps-system
```

## Configuration Reference

### Global

| Parameter | Description | Default |
|-----------|-------------|---------|
| `global.image.registry` | Container image registry | `""` |
| `global.image.tag` | Default image tag | `"5.2.0"` |

### Workers

| Parameter | Description | Default |
|-----------|-------------|---------|
| `workers.minReplicas` | Minimum worker replicas | `2` |
| `workers.maxReplicas` | Maximum worker replicas (HPA) | `20` |
| `workers.concurrency` | Concurrent jobs per worker | `4` |
| `workers.metricsPort` | Prometheus metrics port | `9091` |
| `workers.hpa.metric` | HPA external metric | `raps_queue_depth` |
| `workers.hpa.targetValue` | Target queue depth per worker | `5` |
| `workers.regions` | Regional worker pools | `[us, emea]` |

### Proxy

| Parameter | Description | Default |
|-----------|-------------|---------|
| `proxy.replicaCount` | Number of proxy replicas | `2` |
| `proxy.service.port` | Proxy service port | `8080` |

### Coordinator

| Parameter | Description | Default |
|-----------|-------------|---------|
| `coordinator.replicaCount` | Coordinator replicas | `2` |

### Dashboard

| Parameter | Description | Default |
|-----------|-------------|---------|
| `dashboard.ingress.enabled` | Enable Ingress for dashboard | `false` |
| `dashboard.ingress.host` | Dashboard hostname | `dashboard.rapscli.xyz` |

### Webhook

| Parameter | Description | Default |
|-----------|-------------|---------|
| `webhook.ingress.enabled` | Enable Ingress for webhook | `false` |
| `webhook.ingress.host` | Webhook hostname | `hooks.rapscli.xyz` |

### Message Bus (Redis)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `messageBus.redis.enabled` | Deploy Redis subchart | `true` |
| `messageBus.redis.external.enabled` | Use external Redis | `false` |
| `messageBus.redis.external.url` | External Redis URL | `""` |

### Monitoring

| Parameter | Description | Default |
|-----------|-------------|---------|
| `monitoring.prometheus.serviceMonitor.enabled` | Create ServiceMonitor | `false` |
| `monitoring.prometheus.serviceMonitor.interval` | Scrape interval | `"30s"` |
| `monitoring.grafana.enabled` | Enable Grafana dashboards | `false` |
| `monitoring.grafana.dashboards.enabled` | Create dashboard ConfigMaps | `false` |

### Security

| Parameter | Description | Default |
|-----------|-------------|---------|
| `sealedSecrets.enabled` | Use Sealed Secrets | `false` |
| `secrets.apsCredentials.clientId` | APS Client ID | `""` |
| `secrets.apsCredentials.clientSecret` | APS Client Secret | `""` |

### CronJobs

| Parameter | Description | Default |
|-----------|-------------|---------|
| `cronJobs` | List of scheduled pipeline jobs | `[]` |

Example:
```yaml
cronJobs:
  - name: nightly-translate
    schedule: "0 2 * * *"
    pipeline: pipelines/nightly.yaml
    concurrencyPolicy: Forbid
    backoffLimit: 2
    activeDeadlineSeconds: 7200
```

## Multi-Tenant Setup

Enable multi-tenant mode to isolate workloads per customer:

```yaml
tenants:
  enabled: true
  list:
    - name: acme
      clientId: "acme-client-id"
      clientSecret: "acme-secret"
      workerReplicas: 3
    - name: globex
      clientId: "globex-client-id"
      clientSecret: "globex-secret"
```

This creates:
- Dedicated `raps-tenant-{name}` namespaces
- Isolated worker deployments per tenant
- NetworkPolicies restricting cross-tenant traffic
- Per-tenant credential secrets

## Upgrading

```bash
helm upgrade raps ./deploy/helm/raps/ --namespace raps-system
```

## Uninstalling

```bash
helm uninstall raps --namespace raps-system
```

Note: Tenant namespaces are not automatically deleted. Remove them manually:
```bash
kubectl delete namespace raps-tenant-acme
```
