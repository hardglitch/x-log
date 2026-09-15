use x_log::{log, log_init};

fn main() {
    log_init!(); // always place to main.rs

    log!("Application started!");
    log!("The answer is {}", 42);

    for i in 0..5 {
        log!("Processing item number: {}", i);
    }
}