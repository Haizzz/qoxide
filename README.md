# Qoxide

A lightweight local job queue built in Rust, backed by SQLite.

## Guiding Principles
- **Simple** - Minimal API surface, easy to understand
- **Fast** - SQLite with WAL mode, indexed queries
- **Predictable** - FIFO ordering, atomic operations

## Features
- In-memory or file-based persistence
- SQLite backend with WAL mode for file-based queues
- Binary payload support (arbitrary `Vec<u8>`)
- Atomic reserve-complete/fail workflow

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
qoxide = "1.0"
```

## Usage

### Basic Example
```rust
use qoxide::QoxideQueue;

// Create an in-memory queue
let mut queue = QoxideQueue::new();

// Add a message
let payload = b"my job data".to_vec();
let message_id = queue.add(payload)?;

// Reserve the next pending message (atomic)
let (id, data) = queue.reserve()?;

// Process the job...

// Mark as complete on success
queue.complete(id)?;

// Or mark as failed to return to pending state
// queue.fail(id)?;
```

### Persistent Queue
```rust
// Create a file-backed queue with WAL mode
let mut queue = QoxideQueue::new_with_path("./my_queue.db");
```

### Queue Inspection
```rust
let sizes = queue.size()?;
println!("Total: {}", sizes.total);
println!("Pending: {}", sizes.pending);
println!("Reserved: {}", sizes.reserved);
println!("Completed: {}", sizes.completed);
```

## API Reference

| Method | Description |
|--------|-------------|
| `QoxideQueue::new()` | Create in-memory queue |
| `QoxideQueue::new_with_path(path)` | Create file-backed queue |
| `add(payload: Vec<u8>)` | Add message, returns message ID |
| `reserve()` | Atomically reserve next pending message |
| `complete(id)` | Mark message as completed |
| `fail(id)` | Return message to pending state |
| `size()` | Get queue size breakdown by state |

## Message States

```
PENDING → RESERVED → COMPLETED
            ↓
         (fail)
            ↓
         PENDING
```

- **Pending**: Message is waiting to be processed
- **Reserved**: Message is being processed by a worker
- **Completed**: Message has been successfully processed

## Behaviour

### Ordering
Messages are processed in FIFO order. `reserve()` always returns the oldest pending message.

### Atomicity
The `reserve()` operation is atomic - it selects and updates the message state in a single SQL statement using `UPDATE ... RETURNING`, preventing race conditions.

### Persistence
- **In-memory** (`:memory:`): Data is lost when the queue is dropped
- **File-backed**: Uses SQLite WAL mode for better concurrent read performance

## Limitations

- **Write contention**: SQLite allows only one writer at a time. Multi-process access works but may block under heavy write load
- **No visibility timeout**: Reserved messages stay reserved forever until explicitly completed or failed. If a worker crashes, messages must be manually recovered
- **No dead letter queue**: Failed messages return to pending state indefinitely
- **No message priorities**: Strictly FIFO ordering
- **No delayed/scheduled messages**: Messages are immediately available
- **No TTL/expiration**: Messages never expire automatically
- **Completed messages are not cleaned up**: The `complete()` method marks messages as completed but doesn't delete them. Requires manual cleanup

## Scaling

### What works well
- High throughput for single-writer scenarios
- Large payloads (up to 1MB+ tested in benchmarks)
- Queue sizes of 100k+ messages

### What doesn't scale
- Multiple concurrent writers (SQLite write lock contention)
- Distributed workers (single SQLite file)
- Very high QPS requirements (>10k/sec may hit SQLite limits)

### Recommendations
- For multi-process: Use one queue per process or implement connection pooling
- For distributed: Consider Redis, RabbitMQ, or other distributed queues
- For high throughput: Batch operations where possible

## Benchmarks

Run benchmarks with:
```bash
cargo bench
```

Benchmarks include:
- `queue_add`: Single message enqueue
- `queue_add_large_payload`: 1MB payload enqueue
- `queue_reserve`: Reserve from queues of 1k, 10k, 100k messages
- `queue_interactions`: Full add→reserve→fail→reserve→complete cycle

## Development

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Roadmap

- [ ] Visibility timeout (auto-return reserved messages after timeout)
- [ ] Retry count / max attempts
- [ ] Dead letter queue (DLQ)
- [ ] Delayed/scheduled messages
- [ ] Priority queues
- [ ] Message TTL / expiration
- [ ] Batch operations
- [ ] Message deduplication
- [ ] Cleanup/purge completed messages

## License

MIT License - see [LICENSE](LICENSE) for details.
