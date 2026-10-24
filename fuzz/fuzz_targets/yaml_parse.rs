//! Fuzz target: parsing arbitrary UTF-8 must never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let source = ansilint_yaml::SourceFile::new("fuzz.yml", text);
        let _ = ansilint_yaml::parse(&source);
    }
});
