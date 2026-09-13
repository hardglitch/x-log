use log::{Log, log};

fn main() {
    let _guard = Log::init(); // log.log , max_size = 10 Mb

    log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }

    // Ensure all logs are flushed to disk.
    // The logger will automatically shutdown when `_guard` goes out of scope,
    // or you can call it manually: Log::shutdown();
}