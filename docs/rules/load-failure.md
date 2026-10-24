# load-failure

- **Profile:** min
- **Tags:** syntax
- **Fixable:** no

A file could not be loaded as UTF-8 text. This covers unreadable files, invalid
UTF-8, and files above the size limit (5 MiB by default).

## What triggers it

- an I/O error while reading the file;
- bytes that are not valid UTF-8 (a UTF-8 BOM is accepted and stripped);
- a file larger than the configured limit.

## Examples

Fails: a file containing the bytes `0xFF 0xFE`.

Passes: any UTF-8 YAML file, including one with a UTF-8 BOM or CRLF line endings.

## Skipping

Not skippable with inline `# noqa`; there is no line to attach it to. The finding
is reported at line 1, column 1.
