#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests;
pub mod backend;

/// The primary entry point for the logging system.
/// This struct should be held in `main` to ensure proper shutdown via its `Drop` implementation.
#[derive(Default)]
pub struct Log;

impl Log {
    /// Initializes the logger with default settings (logs to `log.log`).
    pub fn init() -> Self {
        let backend = backend::LogBackend::new();
        backend.init();
        Self
    }

    /// Initializes the logger with a custom file path and maximum file size.
    pub fn init_with<T: AsRef<str>>(path: T, max_size: u64) -> Self {
        let backend = backend::LogBackendBuilder::new()
            .path(path)
            .max_size(max_size)
            .build();
        backend.init();
        Self
    }

    /// Initializes the logger using a pre-configured `LogBackend`.
    pub fn init_with_backend(backend: backend::LogBackend) -> Self {
        backend.init();
        Self
    }

    /// Sends a message to the background thread via a lazy closure.
    /// This is efficient as string formatting happens in the worker thread.
    #[inline]
    pub fn send<F>(f: F)
    where
        F: FnOnce() -> String + Send + 'static
    {
        if let Some(tx) = backend::LOG_SENDER.get() {
            let cmd = backend::LogCommand::Message(Box::new(f));
            let _ = tx.send(cmd);
        }
    }

    /// Sends a pre-formatted string to the background thread.
    /// Use this when the log message depends on data that must be captured immediately in the current thread.
    #[inline]
    pub fn send_owned(msg: String) {
        if let Some(tx) = backend::LOG_SENDER.get() {
            let cmd = backend::LogCommand::MessageOwned(msg);
            let _ = tx.send(cmd);
        }
    }

    /// Manually shuts down the logger and flushes all pending logs.
    pub fn shutdown() {
        backend::LogBackend::shutdown();
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        // Automatically triggers shutdown when the `Log` instance is dropped.
        backend::LogBackend::shutdown();
    }
}

/// Macro for lazy logging. The expression inside will be evaluated in the background thread.
///
/// # Example
/// ```
/// log!("User {} logged in", user_id);
/// ```
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::Log::send(move || format!($($arg)*)); };
}

/// Macro for eager logging. The expression is evaluated immediately in the current thread.
/// Use this when passing references to local variables that won't live long enough for the worker thread.
///
/// # Example
/// ```
/// log_eager!("Current time: {:?}", std::time::Instant::now());
/// ```
#[macro_export]
macro_rules! log_eager {
    ($($arg:tt)*) => { $crate::Log::send_owned(format!($($arg)*)); };
}

// Initialization macros for convenience in `main` functions.

/// Macro to initialize the default logger and bind it to a variable guard.
#[macro_export]
macro_rules! log_init {
    () => { let _guard = $crate::Log::init(); };
}

/// Macro to initialize the logger with a path and max size.
#[macro_export]
macro_rules! log_init_with {
    ($($arg:tt)*) => { let _guard = $crate::Log::init_with($($arg)*); };
}

/// Macro to initialize the logger using a custom `LogBackendBuilder`.
#[macro_export]
macro_rules! log_init_with_backend {
    ($($arg:tt)*) => { let _guard = $crate::Log::init_with_backend($($arg)*); };
}