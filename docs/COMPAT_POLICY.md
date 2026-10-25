# Compatibility policy

Status: decided before Phase 2 (2026-10-23).

## Scope

Tag names, sub-tags, profiles and config keys are the machine-readable contract and
must match upstream exactly. Messages are the human-readable contract: CI users
notice them in diffs, and editor users see them next to code, so we treat them as
stable API too (ENG-30).

## Decision

1. **Tags, sub-tags and profiles:** verbatim upstream identifiers. A rule page and
   `parity.toml` record the source of the mapping.
2. **Messages:** use the upstream wording **verbatim** whenever we have verified
   it from a real `ansible-lint` run. We do not invent a "better" sentence for a
   message that users may already match in CI dashboards.
3. **Dynamic messages** (lists of supported parameters, file paths, variable
   names) keep the upstream prefix and shape, but the generated list is built
   from our index/model. If the wording must differ, the rule page has a
   "Differences from upstream" section and the snapshot test pins our text.
4. **No copied prose:** rule documentation is written in our own words. Only the
   short, factual diagnostic messages are reused for compatibility, as allowed by
   the specification's licensing checklist.
5. **Stability:** message changes are user-visible changes. They require a
   snapshot update, a CHANGELOG entry and a note in the rule page.

## Verified message samples

Captured with `ansible-lint 26.10.1` (ansible-core 2.21.5) on
`ansible/ansible-examples` and `geerlingguy/ansible-role-nginx` on 2026-10-23.

| Tag | Upstream message |
|---|---|
| `name[casing]` | `All names should start with an uppercase letter.` |
| `name[missing]` | `All tasks should be named.` |
| `name[play]` | `All plays should be named.` |
| `yaml[truthy]` | `Truthy value should be one of [false, true]` |
| `yaml[indentation]` | `Wrong indentation: expected 4 but found 3` |
| `yaml[empty-lines]` | `Too many blank lines (2 > 0)` |
| `yaml[brackets]` | `Too many spaces inside brackets` |
| `yaml[braces]` | `Too many spaces inside braces` |
| `yaml[colons]` | `Too many spaces after colon` |
| `yaml[commas]` | `Too few spaces after comma` |
| `yaml[comments]` | `Missing starting space in comment` |
| `no-changed-when` | `Commands should not change things if nothing needs doing.` |
| `command-instead-of-shell` | `Use shell only when shell functionality is required.` |
| `deprecated-local-action` | `Do not use 'local_action', use 'delegate_to: localhost'.` |
| `fqcn[action-core]` | `Use FQCN for builtin module actions (service).` |
| `no-free-form` | `Avoid using free-form when calling module actions. (service)` |
| `package-latest` | `Package installs should not use latest.` |
| `risky-file-permissions` | `File permissions unset or incorrect.` |

These samples are the seed for Phase 2 snapshots. They are not a complete parity
claim: the compatibility harness (Phase 3/8.3) is the authoritative comparison.

## How to add a message

1. Run the reference linter on a minimal fixture and capture the exact message.
2. Record it in the rule's `docs/rules/<id>.md` page under "Upstream message".
3. Add a snapshot test for our message.
4. If we must differ, write the reason in the page and in `docs/TECH_DEBT.md` if
   the difference is temporary.
