use std::fs::{File, OpenOptions};
use std::io::Write;
use std::fmt::Arguments;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub static LOG_FILE: OnceLock<Arc<Mutex<LogFile>>> = OnceLock::new();

pub struct LogFile {
    path: PathBuf,
    size: u64,
    file: File,
}
impl LogFile {
	#[inline]
	fn write_entry(&mut self, args: Arguments) {
		let ts = Log::get_timestamp();
        let _ = self.file.write_all(format!("[{}]: ", ts).as_bytes());
        let _ = self.file.write_fmt(args);
        let _ = self.file.write_all(b"\n");
    }
}

pub struct Log;
impl Log {
    pub fn init(path: &str, file_size: u64) {
        let log_path = PathBuf::from(&path);

        if let Some(dir) = log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path);

        match file {
            Ok(file) => {
                let log_file = LogFile {
                    path: log_path,
                    size: file_size,
                    file,
                };
                LOG_FILE.get_or_init(|| Arc::new(Mutex::new(log_file)));
            },
            Err(e) => { eprintln!("{e}") }
        }
    }

    fn re_init(old_log_path: PathBuf) {
        let old_log_p =
            if let Some(p) = old_log_path.to_str() && !p.is_empty() { p }
            else { return };

        let p =
            if let Some(p) = Self::create_new_name(old_log_p) { p }
            else { return };
        let new_log_path = PathBuf::from(p);

        if let Some(dir) = PathBuf::from(&old_log_path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }

        let new_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&new_log_path);

        match new_file {
            Ok(new_file) =>
                if let Some(log_file) = LOG_FILE.get().cloned() &&
                   let Ok(mut old_log_file) = log_file.try_lock()
                {
                    let new_log_file = LogFile {
                        path: new_log_path,
                        size: old_log_file.size,
                        file: new_file,
                    };
                    *old_log_file = new_log_file;
                }
            Err(e) => { eprintln!("{e}"); }
        }
    }
	
    fn create_new_name(old_name: &str) -> Option<String> {
        let (base, ext) = old_name.rsplit_once(".")?;
        let base_wo_time = if let Some(b) = base.rsplit_once("-").map(|x| x.0) { b } else { base };

        let timestamp = {
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(ts) => ts,
                Err(e) => {
                    eprintln!("{e}");
                    return None
                }
            }
                .as_nanos().to_string()
        };

        let new_name = format!("{base_wo_time}-{timestamp}.{ext}");
        Some(new_name)
    }
	
	#[inline]
    fn get_timestamp() -> String {
        let now = time::OffsetDateTime::now_utc();
        now.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "0000-00-00 00:00:00".to_string())
    }
	
	#[inline]
    pub fn logic(args: Arguments) {
        let mut meta_len = 0_u64;
        let mut log_file_size = 0_u64;

        if let Some(m) = LOG_FILE.get() &&
		   let Ok(log_file) = m.try_lock() &&
           let Ok(meta) = log_file.file.metadata()
		{
            meta_len = meta.len();
            log_file_size = log_file.size;
        }

        if meta_len > log_file_size {
            let mut old_log_path = PathBuf::new();

            if let Some(m) = LOG_FILE.get() &&
	           let Ok(log_file) = m.try_lock()
            {
                old_log_path = log_file.path.clone();
            }
            Log::re_init(old_log_path);
        }

        if let Some(m) = LOG_FILE.get() &&
		   let Ok(mut log_file) = m.try_lock()
		{
			log_file.write_entry(args);
        }
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{ $crate::Log::logic(format_args!($($arg)*)); }}
}



#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[test]
    fn test_stat1_pos() {
        Log::init("./log.log", 1024 * 1024);
        let v1 = 74;
        log!("This value 1 - {}", v1);
        let v2 = 23;
        log!("This value 2 - {}", v2);
    }

    #[ignore]
    #[test]
    fn test_stat2_pos() {
        Log::init("./log.log", 10);
        let v1: usize = 75;
        log!("This value 1 - {}", v1);
        let v2: usize = 24;
        log!("This value 2 - {}", v2);
        let v3: usize = 38;
        log!("This value 3 - {}", v3);
    }
}
