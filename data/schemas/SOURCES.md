# Vendored JSON schemas

No schemas are vendored yet. Vendoring lands in Phase 6 (P6-T1).

Every schema added here must record, in this file:

- origin URL and upstream project;
- the exact commit or release tag it was taken from;
- its license (and a note if the license is not compatible with this project);
- the fetch date;
- the `cargo xtask update-schemas` output it came from.

If a schema's license cannot be confirmed as compatible, it must not be vendored;
fetch it into the user cache at build time instead (OPEN_QUESTIONS #9).
