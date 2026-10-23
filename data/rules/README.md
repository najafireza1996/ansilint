# Static rule data

Static facts used by rules live here as TOML, never duplicated in Rust code
(ENG-15). Each file is authored for this project and carries a provenance comment
(upstream reference, date, and who verified it). Do not copy tables from
upstream source without checking their license.

Planned files (Phase 3 and Phase 5):

- `command_modules.toml` — command → module hints for `command-instead-of-module`
- `package_manager_modules.toml` — for `package-latest`
- `file_creating_modules.toml` — for `risky-file-permissions`
- `vcs_modules.toml` — for `latest`
