#![no_main]

use libfuzzer_sys::fuzz_target;
use log::{Log, log};
use arbitrary::Arbitrary;
use std::sync::Once;
use log::backend::LogBackendBuilder;

static INIT: Once = Once::new();

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    text: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
	INIT.call_once(|| {
        let backend = LogBackendBuilder::new()
            .path("fuzz_test.log")
            .max_size(100 * 1024 * 1024)
            .buffer_size(32 * 1024)
            .channel_size(500)
            .build();

        Log::init_with(backend);
    });
	
	let msg = String::from_utf8(input.text).unwrap_or_default();
    log!("{msg}");
});