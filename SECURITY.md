# Security policy

## Supported versions

`ansilint` is pre-alpha and has no supported release yet. Security fixes will be
provided for the latest published release once `v0.1.0` exists.

## Reporting a vulnerability

Please use GitHub's **private vulnerability reporting** for this repository
(Security → Advisories → Report a vulnerability). Do not open a public issue for a
suspected vulnerability.

Include, when possible:

- the affected version or commit;
- a minimal reproduction (input file or repository layout);
- the observed and expected behavior;
- any potential impact (crash, denial of service, path escape, code execution).

You can expect an acknowledgement within 72 hours. We will coordinate a fix and a
disclosure date with you.

## Scope

The linter processes untrusted YAML. Crashes, panics, unbounded memory use and
path traversal are security issues. The project never executes Ansible code, never
evaluates Jinja, and never accesses the network during normal operation.
