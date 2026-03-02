# RAPS Deployment

Deployment configurations for running RAPS in production environments.

## Deployment Options

### Docker Compose (Development / Small Scale)

Single-host deployment with Redis, workers, proxy, webhook gateway, and dashboard.

```bash
cp .env.example .env
# Edit .env with your APS credentials
docker compose up -d
```

See `docker-compose.yml` for the full stack definition.

| Service | Port | Description |
|---------|------|-------------|
| redis | 6379 | Message broker + cache (Redis 7, AOF persistence) |
| raps-worker (x4) | — | Job consumers (Redis Streams) |
| raps-proxy | 8080/443 | Reverse proxy with HTTPS termination |
| raps-webhook | 9000 | APS webhook receiver |
| raps-dashboard | 3000 | Monitoring dashboard |

```bash
# Scale workers
docker compose up -d --scale raps-worker=8
```

### Fly.io (Serverless)

Scale-to-zero serverless deployment. See `fly.toml`.

```bash
fly deploy
```

### Kubernetes (Enterprise)

Production-grade Kubernetes deployment with:
- Helm chart with Bitnami Redis subchart
- HPA auto-scaling based on queue depth (2-20 pods)
- Regional worker pools (US/EMEA)
- Multi-tenant namespace isolation
- NetworkPolicies and RBAC
- Sealed Secrets for credential management
- Prometheus ServiceMonitor + Grafana dashboards
- CronJobs for scheduled pipelines

```bash
helm install raps ./helm/raps/ \
  --namespace raps-system --create-namespace \
  --set secrets.apsCredentials.clientId=YOUR_ID \
  --set secrets.apsCredentials.clientSecret=YOUR_SECRET
```

See [helm/raps/README.md](helm/raps/README.md) for the full configuration reference.

## CI/CD Examples

- **GitHub Actions**: `examples/github-actions/translate-models.yml`
- **GitLab CI**: `examples/gitlab-ci/raps-pipeline.yml`
- **GitHub Actions (K8s)**: `examples/github-actions/k8s-deploy.yml`
