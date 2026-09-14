#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests;
pub mod backend;

#[derive(Default)]
pub struct Log;

impl Log {
    pub fn init() -> Self {
        let backend = backend::LogBackend::new();
        backend.init();
        Self
    }

    pub fn init_with(backend: backend::LogBackend) -> Self {
        backend.init();
        Self
    }

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

    #[inline]
    pub fn send_owned(msg: String) {
        if let Some(tx) = backend::LOG_SENDER.get() {
            let cmd = backend::LogCommand::MessageOwned(msg);
            let _ = tx.send(cmd);
        }
    }

    #[inline]
    pub fn shutdown() {
        backend::LogBackend::shutdown();
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        backend::LogBackend::shutdown();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{ $crate::Log::send(move || format!($($arg)*)); }};
}

#[macro_export]
macro_rules! log_owned {
    ($($arg:tt)*) => {{ $crate::Log::send_owned(format!($($arg)*)); }};
}