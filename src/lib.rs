#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::arithmetic_side_effects)]
mod tests;

use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub static LOG_FILE: OnceLock<Arc<Mutex<LogFile>>> = OnceLock::new();

#[derive(Debug)]
pub struct LogFile {
    path: PathBuf,
    max_size: u64,
    writer: BufWriter<File>,
}
impl LogFile {
	
    fn rotate(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;

        if let Some(new_path) = self.get_new_path() {
			std::fs::rename(&self.path, new_path)?;
		}

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        
        self.writer = BufWriter::new(file);
        Ok(())
    }

	#[inline]
	fn get_new_path(&self) -> Option<PathBuf> {
        if let Some(stem) = self.path.file_stem().and_then(|s| s.to_str()) {
            let ext = self.path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("log");
				
            if let Ok(ts) = SystemTime::now().duration_since(UNIX_EPOCH) {
				let ts = ts.as_nanos();
				let new_path = self.path.with_file_name(format!("{stem}-{ts}.{ext}"));
				return Some(new_path)
			}
        }
		None
	}

	#[inline]
    fn write_entry(&mut self, args: std::fmt::Arguments) {
        let ts = Self::get_timestamp();
        let _ = write!(self.writer, "[{}]: ", ts);
        let _ = self.writer.write_fmt(args);
        let _ = self.writer.write_all(b"\n");
		let _ = self.writer.flush();
    }

	#[inline]
    fn get_timestamp() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "0000-00-00 00:00:00".to_string())
    }
}

pub struct Log;

impl Log {
	
	#[allow(clippy::expect_used)]
    pub fn init(path: &str, max_size: u64) {
        let path = PathBuf::from(path);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
			// Only once used at the programm start
            .expect("Failed to open log file");

        let log_file = LogFile {
            path,
            max_size,
            writer: BufWriter::new(file),
        };

        LOG_FILE.get_or_init(|| Arc::new(Mutex::new(log_file)));
    }

	#[inline]
    pub fn write(args: std::fmt::Arguments) {
		if let Some(mutex) = LOG_FILE.get() &&
           let Ok(mut log_file) = mutex.lock()
	    {
			if let Ok(metadata) = log_file.writer.get_ref().metadata() &&
			   metadata.len() >= log_file.max_size
			{
				let _ = log_file.rotate();
			}
			log_file.write_entry(args);
        }
    }
	
	pub fn write_async(args: String) -> WriteLogFuture {
        WriteLogFuture { args }
    }
}


pub struct WriteLogFuture {
    args: String,
}
impl Future for WriteLogFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Log::write(format_args!("{}", self.args));
        Poll::Ready(())
    }
}


#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{ $crate::Log::write(format_args!($($arg)*)); }}
}

#[macro_export]
macro_rules! async_log {
    ($($arg:tt)*) => {{ $crate::Log::write_async(format!($($arg)*)).await; }};
}