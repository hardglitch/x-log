use super::*;
use std::fs;

fn cleanup(path: &str) {
	let _ = fs::remove_file(path);
	let start = format!("{}-", path.split('.').next().unwrap());
	for entry in fs::read_dir(".").unwrap() {
		let path = entry.unwrap().path();
		if path.extension().and_then(|s| s.to_str()) == Some("log") &&
           path.file_name().unwrap().to_str().unwrap().starts_with(&start)
	    {
			let _ = fs::remove_file(path);
		}
	}
}

fn read_main_log(path: &str) -> String {
	std::fs::read_to_string(path).unwrap_or_default()
}

fn read_other_logs(path: &str) -> Vec<PathBuf> {
	let start = format!("{}-", path.split('.').next().unwrap());
	fs::read_dir(".")
		.unwrap()
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.filter(|p| p.extension().and_then(|s| s.to_str()) == Some("log") && p.file_name().unwrap().to_str().unwrap().starts_with(&start))
		.collect()
}

#[test]
fn test_stat1_pos() {
	let log_path = "test1.log";
	cleanup(log_path);

	Log::init(log_path, 90); 

	log!("V1: {}", 75);
	log!("V1: {}", 75);
	log!("V1: {}", 75);

	log!("V2: {}", 24);
	log!("V2: {}", 24);
	log!("V2: {}", 24);

	log!("V3: {}", 38);
	log!("V3: {}", 38);
	log!("V3: {}", 38);

	let main_log = read_main_log(log_path);
	assert!(main_log.contains("V3: 38"));
	
	let other_logs = read_other_logs(log_path);
	assert!(!other_logs.is_empty());

    assert_eq!(other_logs.len() + 1, 3);

	let old_log = fs::read_to_string(&other_logs[0]).unwrap();
	assert!(old_log.contains("V1: 75"));

	let old_log = fs::read_to_string(&other_logs[1]).unwrap();
	assert!(old_log.contains("V2: 24"));
	
	cleanup(log_path);
}