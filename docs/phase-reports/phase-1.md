# Phase 1 report (Phases 0, 1 and 1.5)

Branch `phase-1.5-batch-a`, based on `main` at `aae0499`. Every acceptance
criterion is mapped to the test or command that proves it. A criterion without
evidence is listed as **open**, not met.

Status on 2026-11-01: `cargo xtask ci` passes locally (macOS, Apple M1, Rust
1.99.1). The phase is **not complete** until the three open gate items at the
end are closed.

## Phase 0 — Bootstrap

| Criterion | Evidence |
|---|---|
| CI green on Linux, macOS, Windows | **Open:** needs this branch pushed and `.github/workflows/ci.yml` run |
| `ansilint --version` works | `cli::version_flag_prints_the_package_version` |
| No warnings | `cargo xtask ci` (clippy `-D warnings`, rustdoc) |
| `cargo xtask ci` passes locally and in CI | Locally: yes. In CI: **open** (see above) |
| `xtask check` fails on each deliberate violation | `check::tests::rejects_a_forbidden_dependency_edge`, `rejects_a_utils_module`, `rejects_an_untracked_pending_marker`, `rejects_io_inside_a_pure_crate` (plus `rejects_panic_abort_profiles`, `rejects_broken_links_in_the_rule_index`, `rejects_fixtures_without_expected_findings`, `parity_statuses_must_match_rule_files`) |

## Phase 1 — Core engine and CLI skeleton

| Criterion | Evidence |
|---|---|
| `parser-error` with correct line:col | `lint::reports_parser_errors_with_a_position`, `parser::tests::reports_the_error_location`, fixtures `parser-error/expected/*.json` |
| Never panics; fuzz clean ≥ 10 min | 10-minute `yaml_parse` run, 13.1 M executions (CHANGELOG, Phase 1); five targets in `.github/workflows/nightly.yml`; local smoke runs of the three new targets after `a9573bb` |
| Config discovery and merge ("lists extend / scalars override") | `config::tests::merges_lists_and_overrides_scalars`, `config_and_paths::discovers_configuration_in_a_parent_directory`, `config_and_paths::relative_config_path_resolves_exclude_paths_from_the_config_location` |
| `# noqa`, ignore file (`skip` / non-skip), `--generate-ignore` | `lint::noqa_silences_skippable_findings`, `lint::noqa_inside_a_scalar_does_not_suppress`, `post::tests::ignore_entries_skip_or_downgrade`, `config_and_paths::ignore_file_can_skip_a_finding`, `config_and_paths::generates_an_ignore_file`, `config_and_paths::generated_ignore_files_round_trip_paths_with_spaces`, `cli::generates_an_ignore_file` |
| Byte-identical output across runs and thread counts | `cli::output_is_identical_across_thread_counts` (1 vs 4 rayon threads); CI step "Test on one thread" |
| 500-file repository < 100 ms | `cargo xtask bench`: 12 ms best of 5 (`docs/BENCHMARKS.md`) |

## Phase 1.5 — Hardening

Each task links the regression test and the commit that made it pass. Where the
behavior was already correct on `main`, the test is a new guard and the
"fails before" proof is a mutation run, recorded in the commit message.

| Task | Regression test(s) | Commit | Failed before the fix? |
|---|---|---|---|
| H-A1 model issues never findings | `lint::model_issues_in_playbooks_and_handlers_never_become_findings` | `b3d0614` | Already fixed on `main`; both tests failed under a reintroduced issue→warning mutation |
| H-A2 keywords, aliases, merge keys | `keywords::tests::every_documented_task_keyword_is_recognised`, `keyword_table_has_no_undocumented_entries`, `build::tests::resolves_an_alias_used_as_a_task`, `finds_the_action_through_a_merge_key`, `treats_every_with_lookup_as_a_keyword` | `ea8d3af` | Yes (5 failed) |
| H-A3 conservative sniffing | `discovery::tests::sniffs_playbooks_from_any_item_and_document`, `never_sniffs_generic_yaml_as_ansible`, `lint::a_playbook_of_imports_is_not_empty` | `6f74e92` | Yes |
| H-A4 false-positive corpus (42 files) | `valid_corpus::valid_corpus_lints_with_zero_findings` | `90409ad` | Yes (missing corpus) |
| H-B1 `tags` semantics + notice | `config_and_paths::tag_selectors_that_match_no_rule_produce_a_notice`, `cli::tags_flag_keeps_only_matching_rules` | `69b81fc` | Yes |
| H-B2 per-document comments | `parser::tests::each_document_owns_only_its_own_comments` | `0f86a48` | Yes |
| H-B3 bounded recursion | `parser::tests::accepts_nesting_up_to_the_documented_limit`, `rejects_hundred_thousand_deep_input_with_a_spanned_error`, `lint::hundred_thousand_deep_nesting_is_a_parser_error_not_a_crash` | `0f44804` | Yes (constant missing); 100 000-deep inputs were already refused |
| H-B4 panic isolation (release) | `panic_isolation::a_panicking_rule_is_reported_and_the_run_continues` (CI: `--release`, 3 OSes) | `ebf374d` | Yes (feature missing); under the abort strategy the build fails |
| H-B5 CRLF / Unicode columns | `line_index::tests::line_col_matches_the_reference_at_every_byte_offset`, `reference_handles_the_named_edge_cases` | `0d164c0` | Implementation was correct; the test catches two mutations, one of which the old property missed |
| H-C1 implicit `.cache/` | `discovery::tests::cache_is_implicit_only_without_configured_exclude_paths`, `config_and_paths::a_target_directory_is_linted_like_any_other` | `e54f449` | Yes |
| H-C2 no silently inert keys | `config::tests::reports_keys_that_are_accepted_but_not_implemented_yet`, `config_and_paths::configuration_keys_without_an_effect_yet_produce_a_notice` | `a983b13` | Yes |
| H-C3 shared loader | `config_and_paths::configuration_files_go_through_the_shared_loader`, `crlf_configuration_and_ignore_files_are_understood` | `f82a785` | Yes (UTF-8/size case) |
| H-C4 path normalization | `config_and_paths::relative_config_path_resolves_exclude_paths_from_the_config_location`, `files_outside_the_project_root_are_reported_relative_not_absolute`, `discovery::tests::normalize_resolves_dots_lexically`, `paths_outside_the_root_get_parent_segments` | `2df22ed` | Yes |
| H-C5 ignore-file layouts | `config_and_paths::finds_the_ignore_file_in_every_documented_layout` | `fce978c` | Already correct; precedence logged as OPEN #21 |
| H-C6 `--generate-ignore` round trip | `ignore::tests::parse_inverts_to_line` (property), `round_trips_paths_with_spaces_hashes_and_quotes`, `config_and_paths::generated_ignore_files_round_trip_paths_with_spaces` | `f6ac4c9` | Yes |
| H-C7 O(1) ignore lookup | Benchmark: 10 000 × 10 000 lookups 185 ms → 0.9 ms (`docs/BENCHMARKS.md`) | `2a57c7b` | Performance-only |
| H-C8 no AST clones | Benchmark: `build()` 185 306 → 11 006 allocations, 14.6 → 3.7 MB (`docs/BENCHMARKS.md`) | `8efd621` | Performance-only |
| H-C9 fixtures execute | `fixtures::every_rule_fixture_matches_its_expected_findings`, `check::tests::rejects_fixtures_without_expected_findings` | `3202588` | Yes |
| H-C10 determinism gate | `cli::output_is_identical_across_thread_counts`; CI "Test on one thread" | `af1b045` | The test pre-existed; the CI step is new |
| H-C11 more fuzz surface | Targets `loader`, `config`, `ignore_file` (+ `yaml_parse`, `model`) | `9b0e78c` | Found the bug fixed in `a9573bb` |
| H-C12 usage errors exit 1 | `cli::usage_errors_exit_with_one` | (on `main`) | Pre-existing |
| H-C13 no-path behavior documented | `docs/CLI.md` | `6778ed8` | Docs |
| H-C14 dependency ADR | `docs/adr/0003-phase-1-dependency-audit.md` | `41172ee` | Docs |
| H-C15 rule contract tests | `contracts::*_contract` via `rule_contract_tests!` | `a0a543a` | Two mutant rules failed it |
| H-C16 rule-doc links | `check::tests::rejects_broken_links_in_the_rule_index` | `bac1df4` | Yes |
| H-D1 `# noqa` on comment tokens | `parser::tests::hashes_inside_block_and_multiline_quoted_scalars_are_not_comments`, `lint::noqa_inside_a_scalar_does_not_suppress` | `3994de1` | Yes |
| H-D2 non-skippable rules | `post::tests::non_skippable_findings_survive_every_drop_request` | `1fb1187` | Yes |
| H-D3 severity only lowered | `post::tests::severity_is_only_ever_lowered` | `1fb1187` | Yes |
| H-D4 bad `kinds` entries | `config_and_paths::invalid_kinds_entries_produce_notices` (and the upstream-shape fix `parses_upstream_kinds_entries`, `0f0d447`) | `9598993` | Yes |
| H-D5 `#[non_exhaustive]` | `compile_fail` doctest on `FileModel` | `e4ede32` | Yes (doctest fails without the attribute) |
| H-D6 `enable_list` `rule[sub]` | `tests::enable_list_sub_tags_map_to_the_owning_rule` | `52f107c` | Mapping pre-existed; sub-tag filtering deferred (OPEN #23) |
| H-D7 honest README/parity | `check::tests::parity_statuses_must_match_rule_files`; `docs/PARITY.md` reads 4 / 52 | `cb46737` | Yes |

### Phase 1.5 acceptance

| Criterion | Evidence |
|---|---|
| H-A4 corpus: 0 findings, part of `xtask ci` | `valid_corpus::valid_corpus_lints_with_zero_findings` (runs in `cargo test`) |
| Each task has a regression test that fails before the fix | Table above (exceptions stated per row) |
| 100 000-deep nesting and 10 MB pathological anchors: finding or clean refusal | `lint::hundred_thousand_deep_nesting_is_a_parser_error_not_a_crash`, `lint::billion_laughs_aliases_are_never_expanded`, `lint::ten_megabyte_input_is_a_clean_refusal` (`724b0c0`) |
| Release-profile panic test on Linux, macOS, Windows | macOS: passes locally. Linux/Windows: **open**, CI step "Panic isolation (release profile)" |
| CI matrix green before tagging `v0.0.1` | **Open** |
| Second independent review (Appendix A.4) with zero open P0/P1 | **Open** |
| This report exists | This file |

## Open gate items

1. Push the branch and get `ci.yml` green on Linux, macOS and Windows (this
   also covers the release-profile panic test on Linux and Windows).
2. Run an independent review session (spec Appendix A.4) on
   `aae0499..HEAD` and resolve every P0/P1 finding.
3. Have the maintainer review `docs/OPEN_QUESTIONS.md` items 18–23, which
   record decisions made during hardening.

Only then may `v0.0.1` be tagged and Phase 2 start.
