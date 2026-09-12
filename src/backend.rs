use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) static LOG_SENDER: OnceLock<SyncSender<LogCommand>> = OnceLock::new();
const BUFFER_SIZE: usize = 64 * 1024; // 64 kb
const CHANNEL_SIZE: usize = 1024;

#[allow(dead_code)]
pub(crate) enum LogCommand {
    Message(String), // for log!
    Deferred(Box<dyn FnOnce() -> String + Send>), // for async_log!

    Update(LogBackend),
    Flush,
}

#[derive(Debug)]
pub(crate) struct LogBackend {
    pub(crate) path: PathBuf,
    pub(crate) max_size: u64,
    pub(crate) writer: BufWriter<File>,
}
impl LogBackend {
    #[allow(clippy::expect_used)]
    pub(crate) fn new(path: PathBuf, max_size: u64) -> Self {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path).expect("Failed to init log backend");

        Self {
            path,
            max_size,
            writer: BufWriter::with_capacity(BUFFER_SIZE, file),
        }
    }

    #[allow(clippy::expect_used)]
    pub(crate) fn init(path: &str, max_size: u64) {
        let (tx, rx) = sync_channel::<LogCommand>(CHANNEL_SIZE);

        let path = PathBuf::from(path);
        std::thread::spawn(move || {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }

            let mut backend = Self::new(path, max_size);
            loop {
                if let Ok(cmd) = rx.recv() {
                    match cmd {
                        LogCommand::Message(s) => backend.write(s),
                        LogCommand::Deferred(f) => backend.write(f()),
                        LogCommand::Update(backend_) => backend = backend_,
                        LogCommand::Flush => backend.flush(),
                    }
                }
            }
        });

        LOG_SENDER.set(tx).expect("Log initialized more than once");
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        if let Some(new_path) = self.get_new_path() {
            std::fs::rename(&self.path, new_path)?;
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;

        self.writer = BufWriter::with_capacity(BUFFER_SIZE, file);
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
        self.check_rotate();
        let _ = writeln!(self.writer, "[{}]: {}", ts, msg);
    }
    #[inline]
    fn get_timestamp() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "0000-00-00 00:00:00".to_string())
    }
    #[inline]
    fn check_rotate(&mut self) {
        // Flush
        if self.writer.buffer().len() >= self.max_size as usize ||
           self.writer.buffer().len() >= self.writer.capacity()
        {
            self.flush();

            // Rotate
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
}
impl Drop for LogBackend {
    fn drop(&mut self) {
        self.flush();
    }
}