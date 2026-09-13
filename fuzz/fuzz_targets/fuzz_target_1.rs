#![no_main]

use libfuzzer_sys::fuzz_target;
use log::{Log, log};
use arbitrary::Arbitrary;
use std::sync::Once;

static INIT: Once = Once::new();

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    text: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
	INIT.call_once(|| {
        Log::init("fuzz_test.log", 100 * 1024 * 1024);
    });
	
	let msg = String::from_utf8(input.text).unwrap_or_default();
    log!(msg);
});