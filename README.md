# Log
A lightweight, thread-safe logging library for Rust that implements automatic file rotation based on file size. When a log file reaches the specified threshold, it is renamed with a high-precision timestamp and a new log file is created.

## Features
*   **Automatic File Rotation**: Automatically rolls over to a new file when the current one exceeds your specified size limit.
*   **Timestamped Backups**: Renames old logs using Unix nanosecond timestamps to ensure unique filenames and prevent overwriting.
*   **Thread-Safe**: Designed with `Arc`, `Mutex`, and `OnceLock` for safe concurrent logging from multiple threads.
*   **Non-blocking Rotation Check**: Uses non-blocking lock attempts (`try_lock`) during the rotation check to minimize performance impact on your application's main logic.


## Quick Start
Call `Log::init` once at the start of your application. You need to provide a file path and the maximum size (in bytes) allowed before rotation occurs.

```rust
use log::{Log, log};

fn main() {
    // Initialize logging to "./logs/app.log" 
    // Limit set to 1MB (1024 * 1024 bytes)
    Log::init("./logs/app.log", 1024 * 1024);

    let user = "Alice";
    let status = "Success";

    // Use the log! macro to write messages
    log!("User: {}, Status: {}", user, status);
}
```

## How it Works (Rotation Logic)

**1.** Size Check: Every time the log! macro is called, the library checks if the current file size exceeds the limit defined during initialization.

**2.** Renaming: If the threshold is reached, the current file (e.g., app.log) is renamed to a timestamped version (e.g., app-1715432100000000000.log).

**3.** New File Creation: A fresh log file is created immediately, and all subsequent logs are written there.

## API Reference

`Log::init(path: &str, file_size: u64)`

Initializes the global logger instance.

* **path**: The relative or absolute path to the log file.
* **file_size**: Max size in bytes before rotation occurs.
* **Note**: If init is called multiple times, only the first call will configure the logger (due to OnceLock).

**Macro:** `log!(...)`
The primary way to write logs. It expands to a call to the internal logic that handles timestamping and file writing.

---

### 💡 Implementation Tips for Users

* **Performance**: The current implementation uses `try_lock`. If multiple threads attempt to rotate the file at the exact same microsecond, some log entries might be skipped or delayed during the rotation window to prevent blocking your main application logic.

* **Directory Creation**: The library automatically creates parent directories if they do not exist when initializing.
