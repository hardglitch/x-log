#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests;
mod backend;

pub struct Log;

impl Log {

    pub fn init(path: &str, max_size: u64) {
        backend::LogBackend::init(path, max_size);
    }

    #[inline]
    pub fn send(args: std::fmt::Arguments) {
        if let Some(tx) = backend::LOG_SENDER.get() {
            let cmd = backend::LogCommand::Message(format!("{}", args));
            let _ = tx.send(cmd);
        }
    }

    #[inline]
    pub fn send_async<F>(f: F)
    where
        F: FnOnce() -> String + Send + 'static
    {
        if let Some(tx) = backend::LOG_SENDER.get() {
            let cmd = backend::LogCommand::Deferred(Box::new(f));
            let _ = tx.send(cmd);
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{ $crate::Log::send(format_args!($($arg)*)); }}
}

#[macro_export]
macro_rules! async_log {
    ($($arg:tt)*) => {{ $crate::Log::send_async(move || format!($($arg)*)); }};
}