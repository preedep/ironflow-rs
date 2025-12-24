# IronFlow-Rs Quick Start Guide

This guide will help you get IronFlow-Rs up and running in 5 minutes.

## Prerequisites

- Docker and Docker Compose (easiest method)
- OR Rust 1.70+ and Redis/RabbitMQ installed locally

## Method 1: Docker Compose (Recommended)

### 1. Start all services

```bash
# Clone the repository
git clone https://github.com/yourusername/ironflow-rs.git
cd ironflow-rs

# Start Redis, Vault, and IronFlow worker
docker-compose up -d

# Check logs
docker-compose logs -f ironflow-worker-redis
```

### 2. Send a test task

```bash
# Install Python Redis client
pip install redis

# Run the example script
python examples/send_task.py
```

### 3. Check results

```bash
# Connect to Redis
docker exec -it ironflow-rs-redis-1 redis-cli

# Check results queue
LRANGE ironflow:results 0 -1
```

## Method 2: Local Development

### 1. Install dependencies

```bash
# macOS
brew install redis
brew services start redis

# Ubuntu/Debian
sudo apt-get install redis-server
sudo systemctl start redis

# Windows (using WSL2)
sudo apt-get install redis-server
sudo service redis-server start
```

### 2. Build and run

```bash
# Clone and build
git clone https://github.com/yourusername/ironflow-rs.git
cd ironflow-rs
cargo build --release

# Copy example config
cp .env.example .env

# Edit .env with your settings (optional, defaults work for local Redis)
# nano .env

# Run the worker
./target/release/ironflow-rs
```

### 3. Send a test task

In another terminal:

```bash
# Install Python Redis client
pip install redis

# Run the example script
python examples/send_task.py
```

## Verify Installation

You should see logs like this:

```json
{
  "event_date_time": "2024-12-24T06:20:00.000Z",
  "log_type": "APP_LOG",
  "app_id": "ironflow-rs",
  "level": "info",
  "message": "Starting worker: worker-001"
}
```

And when processing tasks:

```json
{
  "event_date_time": "2024-12-24T06:20:05.000Z",
  "log_type": "APP_LOG",
  "correlation_id": "example-batch-001",
  "level": "info",
  "execution_time": 125,
  "message": "Task completed successfully"
}
```

## Common Tasks

### Send a command execution task

```python
import redis
import json
import uuid

r = redis.Redis(host='localhost', port=6379)

task = {
    "id": str(uuid.uuid4()),
    "task_type": {
        "type": "execute_command",
        "command": "echo",
        "args": ["Hello, IronFlow!"]
    },
    "priority": 0,
    "max_retries": 3
}

r.lpush('ironflow:tasks', json.dumps(task))
```

### Check task results

```python
import redis
import json

r = redis.Redis(host='localhost', port=6379)

# Get latest result
result_json = r.rpop('ironflow:results')
if result_json:
    result = json.loads(result_json)
    print(f"Task ID: {result['task_id']}")
    print(f"Status: {result['status']}")
    print(f"Exit Code: {result.get('exit_code')}")
    print(f"Output: {result.get('stdout')}")
```

### Monitor the worker

```bash
# Using Docker
docker-compose logs -f ironflow-worker-redis

# Local installation
# Logs are output to stdout in JSON format
./target/release/ironflow-rs | jq .
```

## Configuration

### Minimal Redis Configuration

```bash
export IRONFLOW_QUEUE__URL=redis://localhost:6379
export IRONFLOW_QUEUE__TASK_QUEUE=ironflow:tasks
export IRONFLOW_QUEUE__RESULT_QUEUE=ironflow:results
```

### With Secret Management (Vault)

```bash
export IRONFLOW_SECRETS__TYPE=hashicorp_vault
export IRONFLOW_SECRETS__URL=http://localhost:8200
export IRONFLOW_SECRETS__TOKEN=your-vault-token
export IRONFLOW_SECRETS__MOUNT_PATH=secret
```

### Adjust Concurrency

```bash
# Process 8 tasks simultaneously
export IRONFLOW_WORKER__CONCURRENCY=8
```

## Troubleshooting

### Worker won't start

1. Check Redis is running:
   ```bash
   redis-cli ping
   # Should return: PONG
   ```

2. Check configuration:
   ```bash
   # Print environment variables
   env | grep IRONFLOW
   ```

3. Check logs for errors:
   ```bash
   docker-compose logs ironflow-worker-redis
   ```

### Tasks not being processed

1. Verify tasks are in the queue:
   ```bash
   redis-cli LLEN ironflow:tasks
   ```

2. Check worker logs for errors

3. Verify task JSON format is correct

### Connection errors

1. For Redis: Check `IRONFLOW_QUEUE__URL` is correct
2. For RabbitMQ: Ensure RabbitMQ is running and accessible
3. For Vault: Verify token and URL are correct

## Next Steps

- Read the full [README.md](README.md) for detailed documentation
- Explore task types: command execution, file transfer, file watching
- Set up secret management for production
- Configure monitoring and alerting
- Deploy to Kubernetes or your preferred platform

## Getting Help

- GitHub Issues: https://github.com/yourusername/ironflow-rs/issues
- Documentation: See README.md
- Examples: Check the `examples/` directory

## Production Checklist

Before deploying to production:

- [ ] Configure proper secret management (Vault/AWS/Azure)
- [ ] Set up TLS/SSL for queue connections
- [ ] Configure appropriate concurrency levels
- [ ] Set up monitoring and alerting
- [ ] Configure log aggregation
- [ ] Set resource limits (CPU/Memory)
- [ ] Test failover and recovery
- [ ] Document runbooks for common issues
- [ ] Set up backup and disaster recovery
- [ ] Configure security scanning and updates
