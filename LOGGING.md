# IronFlow-Rs Logging Guide

## Overview

IronFlow-Rs uses professional-grade logging with `log` and `pretty_env_logger` crates to provide comprehensive debugging and monitoring capabilities.

## Log Levels

The application supports standard Rust log levels:

- **ERROR** - Critical errors that need immediate attention
- **WARN** - Warning messages for potential issues
- **INFO** - General informational messages about application flow
- **DEBUG** - Detailed debugging information
- **TRACE** - Very detailed trace information for deep debugging

## Environment Configuration

Set the `RUST_LOG` environment variable to control log levels:

```bash
# Show all logs at INFO level and above
export RUST_LOG=info

# Show all logs at DEBUG level and above
export RUST_LOG=debug

# Show all logs including TRACE
export RUST_LOG=trace

# Filter by module (show only ironflow_rs logs at debug level)
export RUST_LOG=ironflow_rs=debug

# Multiple filters (debug for ironflow_rs, info for everything else)
export RUST_LOG=ironflow_rs=debug,info

# Very detailed logging for specific modules
export RUST_LOG=ironflow_rs::infrastructure::secrets=trace
```

## Running with Logging

```bash
# Basic run with INFO level
RUST_LOG=info cargo run

# Debug mode for development
RUST_LOG=debug cargo run

# Trace mode for deep debugging
RUST_LOG=trace cargo run

# Production mode (errors and warnings only)
RUST_LOG=warn cargo run
```

## Log Output Examples

### Startup Logs
```
INFO  ironflow_rs > 🚀 IronFlow-Rs starting up...
DEBUG ironflow_rs > Loading configuration from environment variables
INFO  ironflow_rs > ✓ Configuration loaded successfully
DEBUG ironflow_rs > Worker ID: worker-1, Concurrency: 10
DEBUG ironflow_rs > Connecting to Redis queue at: redis://localhost:6379
INFO  ironflow_rs > ✓ Connected to Redis queue
INFO  ironflow_rs > 🎯 Starting worker main loop
```

### Task Processing Logs
```
INFO  ironflow_rs::application::worker > 📥 Received task: 550e8400-e29b-41d4-a716-446655440000 (type: ExecuteCommand)
DEBUG ironflow_rs::application::worker > Task details: priority=5, retry_count=0/3
INFO  ironflow_rs::application::task_processor > ⚙️  Processing task: 550e8400-e29b-41d4-a716-446655440000
DEBUG ironflow_rs::application::task_processor > 🔧 Executing command: "echo" ["Hello World"]
DEBUG ironflow_rs::application::task_processor > ✓ Command executed successfully
INFO  ironflow_rs::application::task_processor > ✅ Task completed: 550e8400-e29b-41d4-a716-446655440000 (duration: 125ms)
INFO  ironflow_rs::application::worker > ✅ Task completed successfully: 550e8400-e29b-41d4-a716-446655440000
```

### Secret Management Logs
```
DEBUG ironflow_rs::infrastructure::secrets::vault_secret > Getting secret: key=database/password
TRACE ironflow_rs::infrastructure::secrets::vault_secret > Parsed key path: path=database, secret_key=password
DEBUG ironflow_rs::infrastructure::secrets::vault_secret > ✓ Secret retrieved successfully: key=database/password
```

### Error Logs
```
ERROR ironflow_rs::application::worker > ❌ Task processing failed: 550e8400-e29b-41d4-a716-446655440000 - Command execution timeout
ERROR ironflow_rs::infrastructure::secrets::aws_secret > Failed to get secret from AWS: secret_name=myapp/api-key, error=ResourceNotFoundException
```

## Logging Features by Component

### Main Application (`src/main.rs`)
- ✅ Startup sequence with emojis
- ✅ Configuration loading
- ✅ Service initialization
- ✅ Connection status for queues and secrets
- ✅ Graceful shutdown messages

### Worker (`src/application/worker.rs`)
- ✅ Worker lifecycle events
- ✅ Task reception with details
- ✅ Task processing status
- ✅ Callback and result queue operations
- ✅ Health check results
- ✅ Graceful shutdown handling

### Task Processor (`src/application/task_processor.rs`)
- ✅ Task type identification
- ✅ Command execution details
- ✅ File transfer progress
- ✅ Composite task sub-task tracking
- ✅ Execution duration and status
- ✅ Secret resolution

### Secret Repositories
- ✅ Connection initialization
- ✅ Secret CRUD operations (get/set/delete)
- ✅ Health check status
- ✅ Error details with context

## Best Practices

1. **Development**: Use `RUST_LOG=debug` for detailed information
2. **Production**: Use `RUST_LOG=info` for normal operations
3. **Troubleshooting**: Use `RUST_LOG=trace` for deep debugging
4. **Performance**: Use `RUST_LOG=warn` or `RUST_LOG=error` for minimal overhead

## Structured Logging

The application uses both:
- **Standard log macros** (`log::info!`, `log::debug!`, etc.) for general logging
- **Custom LogBuilder** for structured application logs with correlation IDs and execution times

## Emoji Guide

- 🚀 Startup/Launch
- ✓ Success
- ❌ Error/Failure
- ⚠️  Warning
- 📥 Receiving/Input
- 📤 Sending/Output
- 🔄 Processing/In Progress
- 🔧 Command Execution
- 📁 File Operations
- 🔐 Secret Operations
- 👀 Watching/Monitoring
- 📦 Composite/Package
- 🎯 Target/Goal
- 👋 Goodbye/Shutdown
- 🛑 Stop/Shutdown Signal
- ⏱️  Timeout
- 📞 Callback

## Troubleshooting

### No logs appearing?
Make sure `RUST_LOG` is set:
```bash
export RUST_LOG=info
cargo run
```

### Too many logs?
Reduce the log level:
```bash
export RUST_LOG=warn
cargo run
```

### Need logs from specific module?
Use module-specific filtering:
```bash
export RUST_LOG=ironflow_rs::infrastructure::secrets=debug,info
cargo run
```
