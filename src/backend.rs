use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global sender for dispatching log commands to the background thread.
pub(crate) static LOG_SENDER: OnceLock<SyncSender<LogCommand>> = OnceLock::new();
/// Global handle to manage the lifecycle of the logging background thread.
static LOG_RECEIVER: OnceLock<Mutex<Option<JoinHandle<()>>>> = OnceLock::new();

const BUFFER_SIZE: usize = 64 * 1024; // 64 kb
const CHANNEL_SIZE: usize = 1024;

/// Commands sent to the background logging thread.
#[allow(dead_code)]
pub(crate) enum LogCommand {
    /// Execute a closure to generate a log string (lazy evaluation).
    Message(Box<dyn FnOnce() -> String + Send>),
    /// Write an already formatted string (eager evaluation).
    MessageOwned(String),
    /// Replace the current backend configuration.
    Update(LogBackend),
    /// Force flush the buffer to disk.
    Flush,
    /// Signal the background thread to finish processing and exit.
    Terminate,
}

/// The core engine responsible for file I/O and log rotation.
#[derive(Debug)]
pub struct LogBackend {
    pub(crate) path: PathBuf,
    pub(crate) max_size: u64,
    pub(crate) writer: BufWriter<File>,
    buffer_size: usize,
    channel_size: usize,
    file_counter: usize,
    file_size: u64,
}

impl LogBackend {
    /// Creates a new default backend writing to `log.log` in the current directory.
    #[allow(clippy::expect_used)]
    pub(crate) fn new() -> Self {
        let path = PathBuf::from("log.log");

        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path).expect("Failed to init log backend");

        Self {
            path,
            max_size: 10 * 1024 * 1024, // Default 10MB
            writer: BufWriter::with_capacity(BUFFER_SIZE, file),
            buffer_size: BUFFER_SIZE,
            channel_size: CHANNEL_SIZE,
            file_counter: 0,
            file_size: 0,
        }
    }

    /// Spawns the background worker thread and initializes global static senders.
    #[allow(clippy::expect_used)]
    pub(crate) fn init(mut self) {
        let (tx, rx) = sync_channel::<LogCommand>(self.channel_size);

        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let handle = std::thread::spawn(move || {
            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        LogCommand::Message(f) => self.write(f()),
                        LogCommand::MessageOwned(msg) => self.write(msg),
                        LogCommand::Update(backend_) => self = backend_,
                        LogCommand::Flush => self.flush(),
                        LogCommand::Terminate => { self.flush(); break }
                    }
                }
            }
        });

        LOG_SENDER.get_or_init(|| tx);
        LOG_RECEIVER.get_or_init(|| Mutex::new(Some(handle)));
    }

    /// Renames the current file with a timestamp and creates a new log file.
    #[inline]
    fn rotate(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;

        if let Some(new_path) = self.get_new_path() {
            std::fs::rename(&self.path, new_path)?;
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;

        self.writer = BufWriter::with_capacity(self.buffer_size, file);
        self.file_counter = self.file_counter.wrapping_add(1);
        self.file_size = 0;

        Ok(())
    }

    /// Generates a unique filename for rotation using timestamp and counter.
    #[inline]
    fn get_new_path(&self) -> Option<PathBuf> {
        let stem = self.path.file_stem()?.to_str()?;
        let ext = self.path.extension().and_then(|e| e.to_str()).unwrap_or("log");
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos();
        let cnt = self.file_counter;
        let new_path = self.path.with_file_name(format!("{stem}-{ts}-{cnt}.{ext}"));
        Some(new_path)
    }

    /// Formats and writes the message to the internal buffer, handling rotation if necessary.
    #[inline]
    pub(crate) fn write(&mut self, msg: String) {
        let ts = Self::get_timestamp();
        // Calculate length including timestamp, separator, and newline
        let msg_len = ts.len().saturating_add(msg.len()).saturating_add(3);

        // Check if the new message exceeds max file size before writing
        if self.file_size.saturating_add(msg_len as u64) >= self.max_size {
            self.flush();
            let _ = self.rotate();
        }

        let _ = writeln!(self.writer, "[{}]: {}", ts, msg);
        self.file_size = self.file_size.saturating_add(msg_len as u64);

        // Proactive flush if buffer is getting full
        if self.writer.buffer().len() >= self.writer.capacity() {
            self.flush();
        }
    }

    /// Returns current UTC time in RFC3339 format.
    #[inline]
    fn get_timestamp() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "0000-00-00 00:00:00".to_string())
    }

    /// Flushes the `BufWriter` to ensure data is written to the OS file buffer.
    #[inline]
    fn flush(&mut self) {
        if !self.writer.buffer().is_empty() {
            let _ = self.writer.flush();
            // Update file size based on actual disk metadata
            if let Ok(metadata) = self.writer.get_ref().metadata() {
                self.file_size = metadata.len();
            }
        }
    }

    /// Gracefully shuts down the logger, flushing all buffers and joining the worker thread.
    #[inline]
    pub(crate) fn shutdown() {
        if let Some(tx) = LOG_SENDER.get() {
            let _ = tx.send(LogCommand::Terminate);
        }
        if let Some(mutex) = LOG_RECEIVER.get() &&
            let Ok(mut guard) = mutex.lock() &&
            let Some(handle) = guard.take()
        {
            let _ = handle.join();
        }
    }
}

impl Drop for LogBackend {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Builder pattern to configure `LogBackend` before initialization.
#[derive(Debug, Default)]
pub struct LogBackendBuilder {
    path: Option<PathBuf>,
    max_size: Option<u64>,
    buffer_size: Option<usize>,
    channel_size: Option<usize>,
}

impl LogBackendBuilder {
    /// Creates a new builder instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the log file path.
    pub fn path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = Some(PathBuf::from(path.as_ref()));
        self
    }

    /// Sets the maximum size of a single log file in bytes before rotation occurs.
    pub fn max_size(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Sets the internal `BufWriter` capacity in bytes.
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    /// Sets the capacity of the command channel (number of pending messages).
    pub fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }

    /// Constructs the `LogBackend` with provided settings or default values.
    #[allow(clippy::expect_used)]
    pub fn build(self) -> LogBackend {
        let path = self.path.unwrap_or(PathBuf::from("log.log"));
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .expect("Failed to init log backend");

        let buf_size = self.buffer_size.unwrap_or(BUFFER_SIZE);

        LogBackend {
            path,
            max_size: self.max_size.unwrap_or(10 * 1024 * 1024),
            writer: BufWriter::with_capacity(buf_size, file),
            buffer_size: buf_size,
            channel_size: self.channel_size.unwrap_or(CHANNEL_SIZE),
            file_counter: 0,
            file_size: 0,
        }
    }
}