use std::io::{self, Write};
use std::process::ExitCode;

use super::run;

/// A writer that rejects every write, standing in for a broken stdout on any OS.
struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout closed"))
    }
}

fn file_with_a_parser_error() -> (tempfile::TempDir, String) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("bad.yml");
    std::fs::write(&path, "a: [1,\n").expect("write fixture");
    let arg = path.to_str().expect("utf-8 temp path").to_owned();
    (directory, arg)
}

#[test]
fn write_failure_maps_to_exit_code_one() {
    let (_directory, path) = file_with_a_parser_error();
    let mut stderr = Vec::new();

    let code = run(["ansilint", path.as_str()], &mut FailingWriter, &mut stderr);

    assert_eq!(code, ExitCode::from(1));
}

#[test]
fn working_writer_on_the_same_input_exits_with_two() {
    let (_directory, path) = file_with_a_parser_error();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());

    let code = run(["ansilint", path.as_str()], &mut stdout, &mut stderr);

    assert_eq!(code, ExitCode::from(2));
    assert!(String::from_utf8_lossy(&stdout).contains("parser-error"));
}

#[test]
fn usage_errors_are_written_to_stderr_and_exit_with_two() {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());

    let code = run(["ansilint", "--no-such-flag"], &mut stdout, &mut stderr);

    assert_eq!(code, ExitCode::from(2));
    assert_eq!(String::from_utf8_lossy(&stdout), "");
    assert!(String::from_utf8_lossy(&stderr).contains("--no-such-flag"));
}

#[test]
fn version_is_written_to_stdout_and_exits_with_zero() {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());

    let code = run(["ansilint", "--version"], &mut stdout, &mut stderr);

    assert_eq!(code, ExitCode::SUCCESS);
    assert!(String::from_utf8_lossy(&stdout).starts_with("ansilint "));
}
