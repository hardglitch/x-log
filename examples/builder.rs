use x_log::backend::{LogBackendBuilder};
use x_log::{Log, log};

fn main() {
    let backend = LogBackendBuilder::new()
        .path("logs/custom.log")
        .max_size(5 * 1024 * 1024)  // 5MB
        .buffer_size(32 * 1024)     // 32KB buffer
        .channel_size(500)          // Queue up to 500 messages
        .build();

    let _guard = Log::init_with(backend);
	
	log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }

    // Ensure all logs are flushed to disk.
    // The logger will automatically shut down when `_guard` goes out of scope,
    // or you can call it manually: Log::shutdown();
}