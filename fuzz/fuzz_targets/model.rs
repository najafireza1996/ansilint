//! Fuzz target: parsing and model building must never panic.
#![no_main]

use ansilint_model::FileKind;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let source = ansilint_yaml::SourceFile::new("fuzz.yml", text);
        if let Ok(documents) = ansilint_yaml::parse(&source) {
            for kind in [
                FileKind::Playbook,
                FileKind::Tasks,
                FileKind::Handlers,
                FileKind::Yaml,
            ] {
                let _ = ansilint_model::build(&documents, kind);
            }
        }
    }
});
