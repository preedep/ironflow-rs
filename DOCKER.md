# Docker Image Comparison

IronFlow-Rs provides multiple Dockerfile options optimized for different use cases.

## 📊 Image Size Comparison

| Dockerfile | Base Image | Final Size | Build Time | Use Case |
|------------|------------|------------|------------|----------|
| `Dockerfile` (default) | Debian bookworm-slim | ~100-120 MB | Fast | **Recommended** - Best compatibility |
| `Dockerfile.alpine` | Alpine 3.19 | ~15-25 MB | Medium | Smallest size, some compatibility issues |
| `Dockerfile.distroless` | Google Distroless | ~30-40 MB | Fast | **Most secure** - No shell, minimal attack surface |

## 🎯 Which One to Use?

### Use `Dockerfile` (Debian) if:
- ✅ You need **maximum compatibility**
- ✅ You're using **dynamic linking** with system libraries
- ✅ You need **debugging tools** (shell, package manager)
- ✅ You're **not concerned** about image size
- ✅ **Default choice** for most users

### Use `Dockerfile.alpine` if:
- ✅ You need the **smallest possible image**
- ✅ You're deploying to **resource-constrained** environments
- ✅ You can handle **musl libc** compatibility
- ⚠️ Requires building with `--target x86_64-unknown-linux-musl`

### Use `Dockerfile.distroless` if:
- ✅ **Security** is your top priority
- ✅ You want **minimal attack surface** (no shell, no package manager)
- ✅ You don't need to debug inside containers
- ✅ You want **smaller than Debian** but **more compatible than Alpine**
- ✅ **Best for production** deployments

## 🔨 Building

### Debian (Default)
```bash
docker build -t ironflow-rs:debian .
```

### Alpine (Smallest)
```bash
docker build -f Dockerfile.alpine -t ironflow-rs:alpine .
```

### Distroless (Most Secure)
```bash
docker build -f Dockerfile.distroless -t ironflow-rs:distroless .
```

## 🚀 Running

All images use the same runtime configuration:

```bash
# Basic run
docker run -e IRONFLOW_QUEUE__URL=redis://redis:6379 ironflow-rs:debian

# With config file
docker run -v $(pwd)/config.toml:/home/ironflow/config.toml \
  -e IRONFLOW_CONFIG_FILE=/home/ironflow/config.toml \
  ironflow-rs:debian

# With environment variables
docker run \
  -e IRONFLOW_WORKER_ID=worker-001 \
  -e IRONFLOW_QUEUE__TYPE=redis \
  -e IRONFLOW_QUEUE__URL=redis://redis:6379 \
  -e IRONFLOW_QUEUE__TASK_QUEUE=ironflow:tasks \
  -e IRONFLOW_SECRETS__TYPE=hashicorp_vault \
  -e IRONFLOW_SECRETS__URL=http://vault:8200 \
  ironflow-rs:debian
```

## 🔐 Security Comparison

| Feature | Debian | Alpine | Distroless |
|---------|--------|--------|------------|
| Shell access | ✅ bash | ✅ sh | ❌ None |
| Package manager | ✅ apt | ✅ apk | ❌ None |
| Non-root user | ✅ Yes | ✅ Yes | ✅ Yes (nonroot) |
| CVE exposure | Medium | Low | **Lowest** |
| Debugging | Easy | Easy | **Difficult** |

## 📦 Multi-Architecture Support

To build for multiple architectures:

```bash
# Setup buildx
docker buildx create --use

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ironflow-rs:latest \
  --push .
```

## 🐳 Docker Compose Example

```yaml
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  ironflow-worker-1:
    image: ironflow-rs:distroless
    depends_on:
      - redis
    environment:
      IRONFLOW_WORKER_ID: worker-001
      IRONFLOW_QUEUE__TYPE: redis
      IRONFLOW_QUEUE__URL: redis://redis:6379
      IRONFLOW_QUEUE__TASK_QUEUE: ironflow:tasks
      IRONFLOW_QUEUE__RESULT_QUEUE: ironflow:results
      RUST_LOG: info
    restart: unless-stopped

  ironflow-worker-2:
    image: ironflow-rs:distroless
    depends_on:
      - redis
    environment:
      IRONFLOW_WORKER_ID: worker-002
      IRONFLOW_QUEUE__TYPE: redis
      IRONFLOW_QUEUE__URL: redis://redis:6379
      IRONFLOW_QUEUE__TASK_QUEUE: ironflow:tasks
      IRONFLOW_QUEUE__RESULT_QUEUE: ironflow:results
      RUST_LOG: info
    restart: unless-stopped
```

## 🔍 Debugging

### Debian/Alpine (with shell)
```bash
# Get shell access
docker exec -it <container-id> /bin/sh

# Check logs
docker logs <container-id>

# Inspect binary
docker exec <container-id> ls -lh /usr/local/bin/ironflow-rs
```

### Distroless (no shell)
```bash
# Can only check logs
docker logs <container-id>

# Use debug variant for troubleshooting
FROM gcr.io/distroless/cc-debian12:debug
# This includes busybox shell
```

## 🎯 Recommendations

**For Development:**
- Use `Dockerfile` (Debian) - easy debugging, full tooling

**For Production:**
- Use `Dockerfile.distroless` - best security, good size
- Use `Dockerfile.alpine` - if size is critical

**For CI/CD:**
- Build all three variants
- Tag appropriately: `ironflow-rs:latest`, `ironflow-rs:alpine`, `ironflow-rs:distroless`

## 📝 Notes

### Alpine Considerations
- Requires musl target: `cargo build --target x86_64-unknown-linux-musl`
- Some crates may have issues with musl
- OpenSSL requires special handling
- DNS resolution differences

### Distroless Considerations
- No shell = harder to debug
- No package manager = can't install tools
- Excellent for security compliance
- Smaller attack surface
- Use `:debug` variant for troubleshooting

### Debian Considerations
- Larger image size
- More CVEs to patch
- Best compatibility
- Easy debugging
- Standard choice
