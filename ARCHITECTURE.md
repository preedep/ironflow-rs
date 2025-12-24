# IronFlow-Rs Architecture

## Overview

IronFlow-Rs is built using Clean Architecture principles, ensuring separation of concerns, testability, and maintainability. The system is divided into three main layers: Domain, Infrastructure, and Application.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         External Systems                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │  Redis   │  │ RabbitMQ │  │  Vault   │  │   AWS    │           │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘           │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────┼───────────────────────────────────────┐
│                    Infrastructure Layer                              │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐ │
│  │     Queues       │  │     Secrets      │  │    Services      │ │
│  │  - Redis         │  │  - Vault         │  │  - Executor      │ │
│  │  - RabbitMQ      │  │  - AWS           │  │  - Transfer      │ │
│  │                  │  │  - Azure         │  │  - Watcher       │ │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘ │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Structured Logging                         │  │
│  │              (Standard App Log Format)                        │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ implements
┌─────────────────────────────┼───────────────────────────────────────┐
│                        Domain Layer                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐ │
│  │    Entities      │  │   Repositories   │  │    Services      │ │
│  │  - Task          │  │  - QueueRepo     │  │  - CommandExec   │ │
│  │  - TaskResult    │  │  - SecretRepo    │  │  - FileTransfer  │ │
│  │  - FileEvent     │  │                  │  │  - FileWatcher   │ │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘ │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Value Objects                               │  │
│  │              (Configuration, Options)                         │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ uses
┌─────────────────────────────┼───────────────────────────────────────┐
│                      Application Layer                               │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                      Worker                                   │  │
│  │  - Receives tasks from queue                                 │  │
│  │  - Manages concurrency with semaphores                       │  │
│  │  - Handles graceful shutdown                                 │  │
│  │  - Performs health checks                                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                  TaskProcessor                                │  │
│  │  - Orchestrates task execution                               │  │
│  │  - Handles different task types                              │  │
│  │  - Manages error handling and retries                        │  │
│  │  - Logs execution with correlation IDs                       │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Layer Descriptions

### Domain Layer

The Domain Layer contains the core business logic and is independent of any external frameworks or libraries.

#### Entities

**Task** - Represents a unit of work to be executed
- Contains task type, priority, retry logic
- Immutable after creation
- Self-validating

**TaskResult** - Represents the outcome of task execution
- Contains status, output, timing information
- Tracks correlation IDs for distributed tracing

**FileEvent** - Represents file system events
- Created, modified, deleted, renamed events
- Contains metadata about the file

#### Repositories (Traits)

**QueueRepository** - Abstract interface for queue operations
- `receive_task()` - Pull tasks from queue
- `send_result()` - Push results to queue
- `acknowledge_task()` - Confirm task processing
- `reject_task()` - Reject and optionally requeue
- `health_check()` - Verify queue connectivity

**SecretRepository** - Abstract interface for secret management
- `get_secret()` - Retrieve a single secret
- `get_secrets()` - Retrieve multiple secrets
- `set_secret()` - Store a secret
- `delete_secret()` - Remove a secret
- `health_check()` - Verify secret store connectivity

#### Services (Traits)

**CommandExecutor** - Execute system commands
- Single execution mode
- Polling mode for long-running processes
- Process status checking

**FileTransferService** - Transfer files between locations
- Support for multiple protocols (FTP, SFTP, S3, Azure)
- Progress tracking
- File operations (list, exists, delete)

**FileWatcherService** - Monitor file system changes
- Pattern matching
- Recursive watching
- Event filtering

### Infrastructure Layer

The Infrastructure Layer provides concrete implementations of domain interfaces.

#### Queue Implementations

**RedisQueueRepository**
- Uses Redis lists for queue operations
- BRPOP for blocking receives
- LPUSH for sending results
- Connection pooling with ConnectionManager

**RabbitMqQueueRepository**
- Uses AMQP protocol
- Prefetch configuration for load balancing
- Acknowledgment tracking
- Durable queues

#### Secret Management Implementations

**VaultSecretRepository**
- HashiCorp Vault KV v2 engine
- Token-based authentication
- Namespace support

**AwsSecretRepository**
- AWS Secrets Manager integration
- IAM-based authentication
- Optional prefix for organization

**AzureSecretRepository**
- Azure Key Vault integration
- Service principal authentication
- Certificate support

#### Service Implementations

**DefaultCommandExecutor**
- Uses Tokio async process spawning
- Captures stdout/stderr
- Timeout support
- Cross-platform (Windows, Linux, macOS)

**OpenDalFileTransferService**
- Built on Apache OpenDAL
- Multi-cloud support
- Streaming for large files
- Progress callbacks

**NotifyFileWatcherService**
- Uses notify crate
- Debouncing support
- Pattern matching with regex
- Event aggregation

#### Logging

**StdAppLog**
- Implements standard-app-log specification
- JSON output format
- Correlation ID tracking
- Structured fields for APM integration

### Application Layer

The Application Layer orchestrates the domain and infrastructure layers.

#### Worker

The Worker is the main entry point that:
1. Initializes all dependencies
2. Creates a semaphore for concurrency control
3. Continuously polls the queue for tasks
4. Spawns async tasks for processing
5. Handles graceful shutdown on SIGTERM/SIGINT
6. Performs periodic health checks

**Concurrency Model:**
```rust
// Semaphore limits concurrent task processing
let semaphore = Arc::new(Semaphore::new(config.concurrency));

loop {
    let permit = semaphore.acquire().await;
    tokio::spawn(async move {
        process_task().await;
        drop(permit); // Release permit when done
    });
}
```

#### TaskProcessor

The TaskProcessor handles task execution:
1. Receives a Task from the Worker
2. Determines task type
3. Delegates to appropriate service
4. Handles errors and retries
5. Returns TaskResult
6. Logs execution with structured logging

**Task Type Handling:**
- **ExecuteCommand**: Calls CommandExecutor
- **ExecutePolling**: Calls CommandExecutor with polling
- **FileTransfer**: Calls FileTransferService
- **FileWatch**: Calls FileWatcherService
- **Composite**: Recursively processes sub-tasks

## Data Flow

### Task Execution Flow

```
1. External System → Queue (Redis/RabbitMQ)
                      ↓
2. Worker.receive_task() ← QueueRepository
                      ↓
3. Worker spawns async task with semaphore permit
                      ↓
4. TaskProcessor.process_task()
                      ↓
5. Match task_type and delegate to service
                      ↓
6. Service executes (CommandExecutor, FileTransfer, etc.)
                      ↓
7. TaskResult created with status, output, timing
                      ↓
8. Worker.send_result() → QueueRepository → Result Queue
                      ↓
9. Worker.acknowledge_task() → QueueRepository
                      ↓
10. Semaphore permit released
```

### Secret Resolution Flow

```
1. Task contains secret reference (e.g., "vault:ssh/key")
                      ↓
2. TaskProcessor.resolve_secret()
                      ↓
3. SecretRepository.get_secret()
                      ↓
4. Infrastructure implementation (Vault/AWS/Azure)
                      ↓
5. Secret value returned
                      ↓
6. Used in task execution (e.g., SSH key for SFTP)
```

## Design Patterns

### Repository Pattern
- Abstracts data access behind interfaces
- Allows swapping implementations (Redis ↔ RabbitMQ)
- Facilitates testing with mocks

### Strategy Pattern
- Different queue strategies (Redis, RabbitMQ)
- Different secret strategies (Vault, AWS, Azure)
- Different transfer strategies (FTP, SFTP, S3)

### Builder Pattern
- LogBuilder for constructing structured logs
- Task builder for creating tasks with defaults

### Dependency Injection
- All dependencies injected via constructors
- Enables testing and flexibility
- Uses Arc<dyn Trait> for shared ownership

## Concurrency Model

### Async/Await with Tokio
- All I/O operations are async
- Non-blocking queue operations
- Efficient resource utilization

### Semaphore-based Concurrency Control
```rust
// Limit concurrent tasks
let semaphore = Arc::new(Semaphore::new(4));

// Acquire permit before processing
let permit = semaphore.acquire_owned().await?;

// Process task
tokio::spawn(async move {
    process_task().await;
    // Permit automatically released on drop
});
```

### Graceful Shutdown
```rust
// Listen for shutdown signal
tokio::select! {
    _ = shutdown_signal.notified() => {
        // Wait for in-flight tasks
        timeout(shutdown_timeout, async {
            semaphore.acquire_many(concurrency).await
        }).await;
    }
    _ = process_next_task() => {}
}
```

## Error Handling

### Error Propagation
- Uses `Result<T, Box<dyn Error + Send + Sync>>`
- Errors bubble up through layers
- Logged at application layer with context

### Retry Logic
- Configurable max retries per task
- Exponential backoff (future enhancement)
- Failed tasks can be requeued

### Health Checks
- Periodic health checks for queue and secrets
- Logged as warnings on failure
- Continues operation (resilient to transient failures)

## Testing Strategy

### Unit Tests
- Each module has `#[cfg(test)]` section
- Mock implementations for testing
- Test business logic in isolation

### Integration Tests
- Marked with `#[ignore]` (require external services)
- Test with real Redis/RabbitMQ
- Verify end-to-end flows

### Test Doubles
- Use `mockall` crate for mocking traits
- Inject test implementations via DI

## Performance Considerations

### Memory Efficiency
- Rust's zero-cost abstractions
- No garbage collection overhead
- Efficient async runtime (Tokio)

### CPU Efficiency
- Async I/O prevents thread blocking
- Semaphore prevents overload
- Minimal allocations in hot paths

### Scalability
- Horizontal: Run multiple workers
- Vertical: Increase concurrency per worker
- Queue-based: Natural load distribution

## Security

### Secret Management
- Never log secret values
- Secrets retrieved on-demand
- Support for multiple secret backends

### Least Privilege
- Worker runs as non-root user
- Minimal required permissions
- Audit logging for all operations

### Network Security
- TLS support for all connections
- Configurable timeouts
- Connection pooling with limits

## Monitoring and Observability

### Structured Logging
- JSON format for machine parsing
- Correlation IDs for distributed tracing
- Execution timing for performance analysis

### Metrics (Future)
- Task processing rate
- Error rate
- Queue depth
- Processing latency

### Tracing (Future)
- OpenTelemetry integration
- Distributed tracing across services
- Span context propagation

## Extension Points

### Adding a New Queue Backend
1. Implement `QueueRepository` trait
2. Add configuration variant
3. Update factory function in main.rs

### Adding a New Secret Backend
1. Implement `SecretRepository` trait
2. Add configuration variant
3. Update factory function in main.rs

### Adding a New Task Type
1. Add variant to `TaskType` enum
2. Implement processing in `TaskProcessor`
3. Add tests

### Adding a New Transfer Protocol
1. Add variant to `FileLocation` enum
2. Update OpenDAL operator creation
3. Add tests

## Future Enhancements

- [ ] Plugin system for custom task types
- [ ] Web UI for monitoring
- [ ] Prometheus metrics exporter
- [ ] OpenTelemetry tracing
- [ ] Task scheduling (cron-like)
- [ ] Task dependencies (DAG execution)
- [ ] Priority queues
- [ ] Dead letter queue
- [ ] Task result persistence
- [ ] Webhook notifications
