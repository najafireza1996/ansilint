//! Fuzz target: arbitrary bytes through the shared decoder (BOM, CRLF,
//! invalid UTF-8), then parsing; line/column lookups at every offset must
//! stay in bounds (H-C11).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some(source) = ansilint_yaml::SourceFile::from_bytes("fuzz.yml", data) else {
        return;
    };
    let text = source.as_str();
    for offset in 0..=u32::try_from(text.len()).unwrap_or(u32::MAX) {
        let position = source.line_index.line_col(text, offset);
        assert!(position.line >= 1 && position.col >= 1);
    }
    let _ = ansilint_yaml::parse(&source);
});
