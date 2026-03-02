# RAPS Distributed Deployment

Docker Compose stack for running RAPS distributed workers with Redis-backed job queues.

## Quick Start

```bash
cd deploy
cp .env.example .env
# Edit .env with your APS credentials

docker compose up -d
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| redis | 6379 | Message broker + cache (Redis 7, AOF persistence) |
| raps-worker (x4) | — | Job consumers (Redis Streams) |
| raps-proxy | 8080/443 | Reverse proxy with HTTPS termination |
| raps-webhook | 9000 | APS webhook receiver |
| raps-dashboard | 3000 | Monitoring dashboard |

## Scaling Workers

```bash
docker compose up -d --scale raps-worker=8
```

## Monitoring

```bash
# Check worker heartbeats
redis-cli keys "raps:worker:heartbeat:*"

# View queue lengths
redis-cli xlen raps:queue:critical
redis-cli xlen raps:queue:normal
redis-cli xlen raps:queue:background

# View dead-letter queue
redis-cli xlen raps:queue:dlq
```
