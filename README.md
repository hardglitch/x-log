# X-Log

A lightweight, asynchronous, and thread-safe logging library for Rust. It offloads string formatting and disk I/O to a dedicated background thread to ensure that logging doesn't block your application's main execution path.

## Features

*   **Asynchronous Logging**: Uses a bounded MPSC channel to move work to a background thread.
*   **Lazy Formatting**: Uses closures to defer string formatting until the background thread is ready, minimizing latency in the caller thread.
*   **Log Rotation**: Automatically rotates log files when they reach a specified size limit.
*   **Thread Safe**: Designed for use across multiple threads via a global static sender.
*   **Buffered I/O**: Uses `BufWriter` to minimize system calls and improve performance.

## Reliability & Coverage
- **Memory Safety**: Verified with `cargo miri` (100% clean).
- **Robustness**: Fuzzed with [`cargo fuzz`](https://github.com/rust-fuzz/cargo-fuzz) (100% clean).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
x-log = "0.7.2"
# or x-log = { git = "https://github.com/hardglitch/log" }
```

## Basic usage

To use the logger, initialize it once at the start of your application.

```rust
use x_log::{Log, log};

fn main() {
    let _guard = Log::init(); // log.log , max_size = 10 MB

    log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }

    // Ensure all logs are flushed to disk.
    // The logger will automatically shutdown when `_guard` goes out of scope (RAII),
    // or you can call it manually: Log::shutdown();
}
```
*logs/app.log*
```
[2026-09-13T13:35:56.6214121Z]: Application started!
[2026-09-13T13:35:56.6214709Z]: The answer is 42
[2026-09-13T13:35:56.6214898Z]: Processing item number: 0
[2026-09-13T13:35:56.6214953Z]: Processing item number: 1
[2026-09-13T13:35:56.6214995Z]: Processing item number: 2
[2026-09-13T13:35:56.6215062Z]: Processing item number: 3
[2026-09-13T13:35:56.6215108Z]: Processing item number: 4
```

## Advanced usage: Customizing via Builder

If you need more control use the `LogBackendBuilder`:

```rust
use x_log::backend::{LogBackendBuilder};
use x_log::{Log, log};

fn main() {
    let backend = LogBackendBuilder::new()
        .path("logs/custom.log")
        .max_size(5 * 1024 * 1024)  // 5MB
        .buffer_size(32 * 1024)     // 32KB buffer
        .channel_size(500)          // Queue up to 500 messages
        .build();

    let _guard = Log::init_with(backend);
	
	log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }

    // Ensure all logs are flushed to disk.
    // The logger will automatically shut down when `_guard` goes out of scope (RAII),
    // or you can call it manually: Log::shutdown();
}
```

## Configuration Details

| Feature | Description | Default |
| :--- | :--- | :--- |
| **Path** | Path to log file | log.log |
| **Max Size** | When a file reaches `max_size`, it is renamed and a new file is created. | 10 Mb |
| **Buffer Size** | Internal buffer size for `BufWriter` (bytes). | 64 KB |
| **Channel Size** | Number of messages that can be queued before the caller blocks. | 1024 |

## Performance Tips

1.  **Avoid Heavy Logic in Macros**: The `log!` macro uses a closure. While this prevents formatting unless the logger is active, try to keep the logic inside the `format!` call simple.
2.  **Shutdown Gracefully**: Always ensure the `Log` instance is dropped or that `Log::shutdown()` is called before your program exits to ensure all buffered messages are flushed to disk.

## License
  * This project is licensed under the [MIT license](LICENSE).
