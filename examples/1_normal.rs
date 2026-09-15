use x_log::{log, log_init_with};

fn main() {
    log_init_with!("logs/normal.log", 1000); // always place to main.rs

    log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }
}