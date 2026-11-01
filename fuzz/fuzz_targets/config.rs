//! Fuzz target: configuration parsing must never panic (H-C11).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(source) = ansilint_yaml::SourceFile::from_bytes(".ansible-lint", data) {
        let _ = ansilint_config::Config::from_bytes(source.as_str());
    }
});
