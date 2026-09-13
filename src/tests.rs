use std::io::BufRead;
use super::*;
use crate::backend::{LogBackendBuilder, LOG_SENDER};
use std::path::PathBuf;

fn cleanup(path: &str) {
	let _ = std::fs::remove_file(path);
	let start = format!("{}-", path.split('.').next().unwrap());
	for entry in std::fs::read_dir(".").unwrap() {
		let path = entry.unwrap().path();
		if path.extension().and_then(|s| s.to_str()) == Some("log") &&
           path.file_name().unwrap().to_str().unwrap().starts_with(&start)
	    {
			let _ = std::fs::remove_file(path);
		}
	}
}

fn read_main_log(path: &str) -> String {
	std::fs::read_to_string(path).unwrap_or_default()
}

fn read_other_logs(path: &str) -> Vec<PathBuf> {
	let start = format!("{}-", path.split('.').next().unwrap());
	std::fs::read_dir(".")
		.unwrap()
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.filter(|p| p.extension().and_then(|s| s.to_str()) == Some("log") && p.file_name().unwrap().to_str().unwrap().starts_with(&start))
		.collect()
}

fn re_init(path: &str, max_size: u64) {
	let backend = LogBackendBuilder::new()
		.path(path)
		.max_size(max_size)
		.build();

	let tx = LOG_SENDER.get().unwrap();

	let cmd = backend::LogCommand::Flush;
	let _ = tx.send(cmd);
	let cmd = backend::LogCommand::Update(backend);
	let _ = tx.send(cmd);
}

#[tokio::test]
async fn test_log_all() {
	let backend = LogBackendBuilder::new()
		.path("test1.log")
		.max_size(0)
		.buffer_size(256)
		.channel_size(50)
		.build();

	let _guard = Log::init_with(backend);

	test_log();
	test_async_log().await;
	stress_test();
}

fn test_log() {
	let log_path = "test1.log";
	cleanup(log_path);
	re_init(log_path, 100);

	log!("V1: {}", 75);
	log!("V1: {}", 75);
	log!("V1: {}", 75);

	log!("V2: {}", 24);
	log!("V2: {}", 24);
	log!("V2: {}", 24);

	log!("V3: {}", 38);
	log!("V3: {}", 38);
	log!("V3: {}", 38);

	if let Some(tx) = LOG_SENDER.get() {
		let cmd = backend::LogCommand::Flush;
		let _ = tx.send(cmd);
	}
	std::thread::sleep(std::time::Duration::from_millis(50));

	let main_log = read_main_log(log_path);
	assert!(main_log.contains("V3: 38"));

	let other_logs = read_other_logs(log_path);
	assert!(!other_logs.is_empty());

    assert_eq!(other_logs.len() + 1, 5);

	let old_log = std::fs::read_to_string(&other_logs[0]).unwrap();
	assert!(old_log.contains("V1: 75"));

	let old_log = std::fs::read_to_string(&other_logs[1]).unwrap();
	assert!(old_log.contains("V2: 24"));

	let old_log = std::fs::read_to_string(&other_logs[3]).unwrap();
	assert!(old_log.contains("V3: 38"));

	cleanup(log_path);
}

async fn test_async_log() {
    let log_path = "test2.log";
    cleanup(log_path);
	re_init(log_path, 50);

    log!("Init: {}", 1);
    
    let mut handles = vec![];
    for i in 2..=7 {
        handles.push(tokio::spawn(async move { log!("Task: {}", i); }));
    }
    for h in handles { h.await.unwrap(); }

	if let Some(tx) = LOG_SENDER.get() {
		let cmd = backend::LogCommand::Flush;
		let _ = tx.send(cmd);
	}
	tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let main_log = read_main_log(log_path);
    let mut total_lines = main_log.lines().count();
	total_lines += read_other_logs(log_path).iter()
		.map(|p| { std::fs::read_to_string(p).unwrap().lines().count() })
		.sum::<usize>();

    assert!(total_lines >= 7);
    cleanup(log_path);
}

fn stress_test() {
	let log_path = "test3.log";
	cleanup(log_path);
	re_init(log_path, 10 * 1024 * 1024);

	for _ in 0..1_000_000 {
		log!("This is a stress test");
	}

	if let Some(tx) = LOG_SENDER.get() {
		let cmd = backend::LogCommand::Flush;
		let _ = tx.send(cmd);
	}

	let mut lines = std::fs::read(log_path).unwrap().lines().count();
	lines += read_other_logs(log_path).iter()
		.map(|p| { std::fs::read(p).unwrap().lines().count() })
		.sum::<usize>();

	assert_eq!(lines, 1_000_000);
	cleanup(log_path);
}
