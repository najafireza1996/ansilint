//! Fuzz target: ignore-file parsing never panics, and every parsed entry
//! survives a render/parse round trip (H-C11, H-C6).
#![no_main]

use ansilint_config::IgnoreFile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(file) = IgnoreFile::parse(text) else {
        return;
    };
    for entry in &file.entries {
        let again = IgnoreFile::parse(&entry.to_line()).expect("rendered line parses");
        let parsed = again.entries.first().expect("one entry");
        assert_eq!((&parsed.path, &parsed.tag, parsed.skip), (&entry.path, &entry.tag, entry.skip));
    }
});
