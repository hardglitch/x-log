use x_log::{log_eager, log_init_with};

fn func(answer: &str) {
    // log!("The answer is {answer}"); <-- This won't compile
    // Use `log_eager!` macro to own data in main thread
    log_eager!("The answer is {answer}");
}

fn main() {
    log_init_with!("logs/normal.log", 1000); // Always place this in main.rs
    func("some_str");
}