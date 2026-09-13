use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) static LOG_SENDER: OnceLock<SyncSender<LogCommand>> = OnceLock::new();
static LOG_RECEIVER: OnceLock<Mutex<Option<JoinHandle<()>>>> = OnceLock::new();
const BUFFER_SIZE: usize = 64 * 1024; // 64 kb
const CHANNEL_SIZE: usize = 1024;

#[allow(dead_code)]
pub(crate) enum LogCommand {
    Message(Box<dyn FnOnce() -> String + Send>),
    Update(LogBackend),
    Flush,
    Terminate,
}

#[derive(Debug)]
pub struct LogBackend {
    pub(crate) path: PathBuf,
    pub(crate) max_size: u64,
    pub(crate) writer: BufWriter<File>,
    buffer_size: usize,
    channel_size: usize,
}
impl LogBackend {
    #[allow(clippy::expect_used)]
    pub(crate) fn new<T: AsRef<str>>(path: T, max_size: u64) -> Self {
        let path = PathBuf::from(path.as_ref());
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path).expect("Failed to init log backend");

        Self {
            path,
            max_size,
            writer: BufWriter::with_capacity(BUFFER_SIZE, file),
            buffer_size: BUFFER_SIZE,
            channel_size: CHANNEL_SIZE,
        }
    }

    #[allow(clippy::expect_used)]
    pub(crate) fn init(mut self) {
        let (tx, rx) = sync_channel::<LogCommand>(self.channel_size);

        let handle = std::thread::spawn(move || {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }

            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        LogCommand::Message(f) => self.write(f()),
                        LogCommand::Update(backend_) => self = backend_,
                        LogCommand::Flush => self.flush(),
                        LogCommand::Terminate => { self.flush(); break }
                    }
                }
            }
        });

        LOG_SENDER.set(tx).expect("Log sender initialized more than once");
        LOG_RECEIVER.set(Mutex::new(Some(handle))).expect("Log receiver initialized more than once");
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        if let Some(new_path) = self.get_new_path() {
            std::fs::rename(&self.path, new_path)?;
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;

        self.writer = BufWriter::with_capacity(self.buffer_size, file);
        Ok(())
    }

    #[inline]
    fn get_new_path(&self) -> Option<PathBuf> {
        let stem = self.path.file_stem()?.to_str()?;
        let ext = self.path.extension().and_then(|e| e.to_str()).unwrap_or("log");
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos();
        let hash = (ts % 1000) as u32;
        let new_path = self.path.with_file_name(format!("{stem}-{ts}-{hash}.{ext}"));
        Some(new_path)
    }

    #[inline]
    pub(crate) fn write(&mut self, msg: String) {
        let ts = Self::get_timestamp();
        let _ = writeln!(self.writer, "[{}]: {}", ts, msg);
        self.check_rotate();
    }
    #[inline]
    fn get_timestamp() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "0000-00-00 00:00:00".to_string())
    }
    #[inline]
    fn check_rotate(&mut self) {
        if self.writer.buffer().len() >= self.max_size as usize ||
           self.writer.buffer().len() >= self.writer.capacity()
        {
            self.flush();

            if let Ok(metadata) = self.writer.get_ref().metadata() &&
               metadata.len() >= self.max_size
            {
                let _ = self.rotate();
            }
        }
    }

    #[inline]
    fn flush(&mut self) {
        let _ = self.writer.flush();
    }

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

#[derive(Debug)]
pub struct LogBackendBuilder {
    pub(crate) path: PathBuf,
    pub(crate) max_size: u64,
    buffer_size: Option<usize>,
    channel_size: Option<usize>,
}
impl LogBackendBuilder {
    pub fn path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.path = PathBuf::from(path.as_ref());
        self
    }
    pub fn max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }
    pub fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);
        self
    }
    #[allow(clippy::expect_used)]
    pub fn build(self) -> LogBackend {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)
            .expect("Failed to init log backend");

        let buf_size = self.buffer_size.unwrap_or(BUFFER_SIZE);

        LogBackend {
            path: self.path,
            max_size: self.max_size,
            writer: BufWriter::with_capacity(buf_size, file),
            buffer_size: buf_size,
            channel_size: self.channel_size.unwrap_or(CHANNEL_SIZE),
        }
    }
}