# Engineering standards

> Canonical copy of Section 12 of the project specification (v1.1, 2026-10-23).
> From now on this file is the source of truth; the specification section is frozen.
> Cite rule IDs (`ENG-xx`) in review comments and commit messages.

## 12. Engineering standards (non-negotiable)

**Purpose:** keep the codebase clean, evolvable and welcoming to outside contributors. These rules apply to humans and AI agents alike.
**How to read this section:** every rule has an ID (`ENG-xx`) so reviews and PR comments can cite it ("violates ENG-22"). The last column says *what enforces it*: a rule that only exists as prose is a suggestion, so **anything marked `review` must be turned into tooling the first time it is violated twice**. If a rule here conflicts with a task elsewhere in this spec, stop, log it in `docs/OPEN_QUESTIONS.md` and propose an ADR (ENG-40) — never silently break either.
**Precedence of values:** correctness (no false positives, no data loss in `--fix`) > readability > consistency > measured performance > brevity. Cleverness is last.
**Source of truth:** this section is the source of truth until it is copied to `docs/ENGINEERING.md` (task P0-T8); after that, `docs/ENGINEERING.md` is canonical and this section is frozen at v1.1.

### 12.1 Architecture rules

| ID | Rule | Enforced by |
|---|---|---|
| ENG-01 | **Layered crates, whitelisted edges.** Allowed dependency edges are listed in `architecture.toml` (Section 12.11). No cycles. Any new edge requires an ADR. `ansilint-testing` may only appear under `[dev-dependencies]`. | `cargo xtask check` (parses `cargo metadata`) |
| ENG-02 | **Functional core, imperative shell.** Crates `core`, `yaml`, `model`, `config` (parsing only), `rules`, `index` (query only), `schema`, `jinja`, `fix` (planning only) and `output` (formatting into a `Write`) perform **no I/O**: no filesystem, env vars, processes, network, clocks, randomness. Real I/O lives only in `engine` (discovery, loading, atomic writes), `cli`, `lsp` and `xtask`. Pure crates receive bytes/strings and return values. | per-crate `clippy.toml` `disallowed-methods`/`disallowed-types` (`std::fs::*`, `std::env::*`, `std::process::*`, `std::net::*`, `std::time::SystemTime::now`, …) |
| ENG-03 | **Dependency inversion.** `rules`, `fix`, `output` depend on traits (`ModuleIndex`, `SchemaProvider`, `Formatter`), never on concrete loaders. Concrete implementations are composed in `engine`. Every trait that abstracts I/O or data has an in-memory implementation in `ansilint-testing`. | review + contract tests (ENG-12) |
| ENG-04 | **Minimal public API.** Default visibility is `pub(crate)`. A `pub` item needs a doc comment and at least one user in another crate. Public enums/structs that may grow are `#[non_exhaustive]`. | `#![deny(unreachable_pub, missing_docs)]`; `cargo-semver-checks` from 1.0 |
| ENG-05 | **No global mutable state.** Only immutable, lazily initialised data (`LazyLock`/`OnceLock`): compiled regexes, schemas, embedded index. No `static mut`, no singletons holding config. | clippy (`static_mut_refs`, `declare_interior_mutable_const`), review |
| ENG-06 | **One-way data flow.** The AST and model are immutable after construction. Rules return findings; fixes are *data* (`Edit`s), never side effects. | types (no `&mut` model in `Rule` trait), review |
| ENG-07 | **Every crate states its responsibility** in one sentence **and what it must not do** (crate-level docs + README). | `missing_docs`, `xtask check` (non-empty crate docs) |
| ENG-08 | **Cross-cutting concerns are explicit.** Logging uses `tracing` and only in `engine`/`cli`/`lsp`; libraries return data or typed errors. Configuration is passed as typed values; no deep code reads env vars or config files. | clippy `print_stdout`/`print_stderr` denied in libs; `disallowed-methods` |
| ENG-09 | **Extension points = trait + registry** (rules, formatters, index layers, schema kinds, fix providers). Adding an instance touches its own file plus one registry line. | review, "add-a-rule in ≤ 3 files" CI check (`xtask new-rule` smoke test) |

### 12.2 SOLID, applied

| ID | Rule | Enforced by |
|---|---|---|
| ENG-10 | **Single responsibility.** One module = one reason to change. One rule = one file. Limits: function ≤ 50 lines, cognitive complexity ≤ 15, nesting depth ≤ 3, parameters ≤ 5 (otherwise a parameter struct/builder), file ≤ 400 lines (warn) / 800 (fail); generated files exempt. | clippy (`too_many_lines`, `cognitive_complexity`, `too_many_arguments`), `xtask check` (file size) |
| ENG-11 | **Open/closed.** No `match rule_id { … }`, and no scattered `if kind == X` chains, outside the registry/generated code. New behavior is added by adding a type that implements a trait. | review; grep-based `xtask check` for `match` on `.id()` outside `registry` |
| ENG-12 | **Liskov.** Every trait documents its contract (preconditions, postconditions, determinism, thread-safety, error behavior) and ships a reusable contract-test macro in `ansilint-testing` that is run against **every** implementation (`Rule`, `Formatter`, `ModuleIndex`, `SchemaProvider`). | contract tests in CI |
| ENG-13 | **Interface segregation.** Traits have ≤ 5 required methods (use default methods or split). No "god context": `FileCx` exposes narrow read-only accessors; a rule uses only what it needs. | review |
| ENG-14 | **Dependency inversion** — see ENG-03. | see ENG-03 |

### 12.3 DRY and its counterweights

| ID | Rule | Enforced by |
|---|---|---|
| ENG-15 | **DRY means one source of truth for *knowledge*.** `RuleMeta` generates `--list-rules/-T/-P`, docs index, parity table and profile tests. The task-keyword table, file-kind patterns and all static facts (command modules, package-manager modules, …) exist in exactly one place; facts live in `data/*.toml`, not scattered across Rust code. | tests that compare generated outputs; review |
| ENG-16 | **Rule of three.** Extract a helper after the third real duplication, not earlier. Put it in the *lowest* crate that all users share, in a module named for what it does (`task_keys`, `span_math`). **Banned module/file names:** `utils`, `util`, `helpers`, `common`, `misc`, `base`, `manager`, `stuff`. | `xtask check` (file/module-name denylist) |
| ENG-17 | **Tests are code too.** Shared builders, fixture loaders and contract macros live in `ansilint-testing`; no copy-pasted loaders. | review |
| ENG-18 | **Macros only for** (a) the rule registry, (b) contract tests, (c) fixture-test generation; each ≤ 50 lines with a documented expansion example. No macro-generated public API otherwise. | review |
| ENG-19 | **Don't DRY unrelated things.** Two rules that look alike but change for different reasons stay separate. Duplication is cheaper than the wrong abstraction; note the decision in the PR when you deliberately keep duplicates. | review |
| ENG-20 | **YAGNI.** No trait with a single implementation unless it is an I/O/testing port (ENG-03) or a listed extension point (ENG-09). No config option without a user story. No generic parameter without two concrete uses. | review |

### 12.4 Clean code

| ID | Rule | Enforced by |
|---|---|---|
| ENG-21 | **Naming.** Follow the Rust API Guidelines. Names reveal intent; abbreviations only if domain-standard (`fqcn`, `yaml`, `ast`, `lsp`, `noqa`). Predicates `is_/has_/should_`; conversions `as_/to_/into_`. Avoid `data`, `info`, `handler`, `process`, `do_…`. | clippy `disallowed_names`, review |
| ENG-22 | **No boolean parameters that switch behavior** (use an enum or two functions). No `Option<bool>`. | clippy `fn_params_excessive_bools`, review |
| ENG-23 | **No primitive obsession / stringly-typed internals.** Newtypes for `RuleId`, `Tag`, `Fqcn`, `FileKind`, byte offsets and line numbers. | review |
| ENG-24 | **Make illegal states unrepresentable; parse, don't validate.** Raw config/YAML is converted to typed structures at the boundary; inner code never re-validates. Use enums with data instead of flag combinations. | review |
| ENG-25 | **Errors.** `thiserror` enums per crate with context; no `String` errors; `?` for propagation; messages lowercase, no trailing period, actionable. **Denied outside tests/xtask/bin:** `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, `println!`/`eprintln!`, `unsafe`. `#[must_use]` on results/builders. | workspace lints (Section 12.10), `forbid(unsafe_code)` |
| ENG-26 | **Comments explain *why*; code explains *what*.** Non-trivial public items have a doctest example. No commented-out code. `TODO` only as `TODO(#<issue>)`; `FIXME`/`XXX`/`HACK` banned. | `xtask check` (regex), review |
| ENG-27 | **Flat control flow.** Early returns, iterator chains over index loops, at most one level of nested closures, no `else` after `return`. | clippy (`needless_return`, `manual_*`), review |
| ENG-28 | **Ownership discipline.** Prefer `&str`, slices, `Cow`, `Arc<str>`. A `.clone()` of a non-`Copy` value on a hot path needs a justification comment. `Rc<RefCell<_>>`/`Arc<Mutex<_>>` require an ADR (justify shared mutability). | clippy (`redundant_clone`, `clone_on_ref_ptr`), review |
| ENG-29 | **Formatting is automated;** no style debates in review. | `cargo fmt --check` |
| ENG-30 | **User-visible output is API.** Messages, tags, exit codes and formats are stable, documented and snapshot-tested. | `insta` snapshots, review |
| ENG-31 | **Exceptions are explicit.** Suppress a lint only with `#[expect(lint, reason = "…")]` (or `#[allow]` + `reason`) scoped to the smallest item. | clippy `allow_attributes_without_reason` |

### 12.5 Performance engineering

| ID | Rule | Enforced by |
|---|---|---|
| ENG-32 | **Measure first.** Optimizations need benchmark evidence linked in the PR. Standard techniques are documented in `docs/PERFORMANCE.md`: read each file once; no per-character allocation; compile regexes once; borrow slices of the source buffer instead of cloning; parallelize at file granularity only. | review, benchmarks |
| ENG-33 | **Budgets are gates:** > 20 % slowdown on the benchmark suite fails CI; stripped release binary size and clean-build time are tracked in `docs/BENCHMARKS.md` and may not regress > 10 % without an ADR. | CI benchmark job |

### 12.6 Testing standards

| ID | Rule | Enforced by |
|---|---|---|
| ENG-34 | Every bug fix starts with a failing regression test. | review |
| ENG-35 | Tests are deterministic and hermetic: no network, no real home directory, temp dirs only, independent of order and thread count. | CI runs with `--test-threads=1` and default; sandboxed `HOME` |
| ENG-36 | Test names describe behavior (`reports_missing_name_on_unnamed_task`); one behavior per test; arrange-act-assert; no loops/conditionals in test bodies (use table-driven helpers from `ansilint-testing`). | review |
| ENG-37 | **Coverage gate:** `cargo-llvm-cov` ≥ 85 % lines for `core`, `yaml`, `model`, `config`, `rules`, `fix`; ≥ 70 % for others; a PR may not reduce coverage. **Mutation testing** (`cargo-mutants`) runs nightly on `core`, `config`, `fix`; surviving mutants become issues. | CI |
| ENG-38 | No `#[ignore]` without an issue link. Snapshot changes require explicit reviewer approval. | `xtask check`, review |

### 12.7 Documentation & decisions

| ID | Rule | Enforced by |
|---|---|---|
| ENG-39 | `#![deny(missing_docs)]` on all library crates; crate docs state responsibility and non-goals; `docs/ARCHITECTURE.md` is updated in the same PR as any structural change. | lint, review |
| ENG-40 | **ADRs** (`docs/adr/NNNN-title.md`: Context, Decision, Consequences, Alternatives) are required for: new crate, new dependency edge, new public trait, new dependency of significance, YAML crate, LSP stack, index format, any exception to a rule in this section. ADRs are immutable; supersede them with a new one. | PR template checkbox, `xtask check` (ADR numbering) |
| ENG-41 | User-visible changes update `CHANGELOG.md` (Keep a Changelog) and docs **in the same PR**. | PR template, review |

### 12.8 Dependencies & features

| ID | Rule | Enforced by |
|---|---|---|
| ENG-42 | **New dependency policy** (answer in the PR): what it does, why `std`/existing deps don't suffice, license, maintenance health, MSRV, `unsafe` usage, compile-time and binary-size impact. No dependency for < 30 lines of trivial code. | `cargo deny` (licenses, advisories, bans, duplicates), `cargo machete` (unused), review |
| ENG-43 | **Features are additive,** documented, never change behavior silently; default features serve ~95 % of users. | review, `cargo hack --feature-powerset` (CI, nightly) |

### 12.9 Git, PRs, review, debt

| ID | Rule | Enforced by |
|---|---|---|
| ENG-44 | Conventional Commits; one logical change per commit; **refactors never mixed with behavior changes** (separate commits, preferably separate PRs). | commit-lint in CI |
| ENG-45 | PR ≤ ≈ 400 changed lines excluding fixtures/snapshots/generated files; larger work is split or discussed first. | CI size label, review |
| ENG-46 | ≥ 1 approving review; `CODEOWNERS` covers `architecture.toml`, all `Cargo.toml`, `crates/ansilint-core/**`, `data/**`, `docs/adr/**`. The sole maintainer may self-merge only with green CI and a 24 h wait. | branch protection |
| ENG-47 | Reviewers use **Appendix D** and cite rule IDs; formatting nitpicks are automated away. | process |
| ENG-48 | **Debt is explicit.** A shortcut requires `TODO(#issue)` plus an entry in `docs/TECH_DEBT.md` (what, why, when to fix, owner). Reviewed quarterly. | `xtask check` (every TODO has an entry) |

### 12.10 Tooling configuration (starting point — adjust, but keep the intent)

**Workspace `Cargo.toml`**
```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "deny"          # library crates; binaries/xtask override locally
unreachable_pub = "deny"
rust_2018_idioms = { level = "deny", priority = -1 }

[workspace.lints.clippy]
all      = { level = "deny",  priority = -1 }
pedantic = { level = "warn",  priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"          # override in cli/xtask only
print_stderr = "deny"
indexing_slicing = "warn"
allow_attributes_without_reason = "deny"
fn_params_excessive_bools = "deny"
too_many_lines = "warn"
cognitive_complexity = "warn"
# Pedantic lints you may allow workspace-wide (document each): module_name_repetitions, must_use_candidate (only if noisy)
```

**`clippy.toml` (workspace root)** — keys must be checked against the clippy version in use (VERIFY):
```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold   = 5
too-many-lines-threshold       = 50
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests  = true
allow-print-in-tests  = true
disallowed-names = ["foo", "bar", "baz", "tmp", "data", "info", "util", "helper", "manager"]
```

**Per-pure-crate `clippy.toml`** (e.g. `crates/ansilint-rules/clippy.toml`) — enforces ENG-02:
```toml
disallowed-methods = [
  { path = "std::fs::read",            reason = "pure crate: I/O belongs to ansilint-engine" },
  { path = "std::fs::read_to_string",  reason = "pure crate: I/O belongs to ansilint-engine" },
  { path = "std::fs::write",           reason = "pure crate: I/O belongs to ansilint-engine" },
  { path = "std::env::var",            reason = "pure crate: pass configuration explicitly" },
  { path = "std::process::Command::new", reason = "pure crate: no subprocesses" },
  { path = "std::time::SystemTime::now", reason = "pure crate: no clocks (determinism)" },
]
disallowed-types = [
  { path = "std::fs::File",     reason = "pure crate: no filesystem" },
  { path = "std::net::TcpStream", reason = "pure crate: no network" },
]
```

### 12.11 `architecture.toml` and `cargo xtask check`

```toml
# architecture.toml — the ONLY allowed intra-workspace dependency edges (ENG-01)
[edges]
ansilint-core    = []
ansilint-yaml    = ["ansilint-core"]
ansilint-model   = ["ansilint-core", "ansilint-yaml"]
ansilint-config  = ["ansilint-core", "ansilint-yaml"]
ansilint-index   = ["ansilint-core"]
ansilint-schema  = ["ansilint-core", "ansilint-yaml"]
ansilint-jinja   = ["ansilint-core"]
ansilint-rules   = ["ansilint-core", "ansilint-yaml", "ansilint-model", "ansilint-config", "ansilint-index", "ansilint-schema", "ansilint-jinja"]
ansilint-fix     = ["ansilint-core", "ansilint-yaml"]
ansilint-output  = ["ansilint-core"]
ansilint-engine  = ["ansilint-core", "ansilint-yaml", "ansilint-model", "ansilint-config", "ansilint-index", "ansilint-schema", "ansilint-rules", "ansilint-fix", "ansilint-output"]
ansilint-lsp     = ["ansilint-core", "ansilint-engine"]
ansilint-cli     = ["ansilint-core", "ansilint-engine", "ansilint-lsp", "ansilint-output"]
# ansilint-testing: dev-dependency only, any crate may use it under [dev-dependencies]

[pure_crates]   # must not touch I/O (ENG-02) — each must contain the disallowed-methods clippy.toml
list = ["ansilint-core","ansilint-yaml","ansilint-model","ansilint-config","ansilint-rules","ansilint-index","ansilint-schema","ansilint-jinja","ansilint-fix","ansilint-output"]
```

`cargo xtask check` (fast, no network, < 10 s; run locally and in CI) must verify:
1. Dependency edges ⊆ `architecture.toml`; no cycles; `ansilint-testing` only in dev-deps.
2. Every pure crate has the I/O-denying `clippy.toml`.
3. No file/module named `utils|util|helpers|common|misc|base|manager`.
4. File-size limits (400 warn / 800 fail, generated exempt).
5. Every `TODO` matches `TODO(#\d+)` and has an entry in `docs/TECH_DEBT.md`; no `FIXME|XXX|HACK`; no `#[ignore]` without issue link.
6. Registry ↔ `parity.toml` ↔ `docs/rules/*.md` ↔ fixtures consistency (every rule has fail/pass/noqa fixtures and a doc page).
7. No `match` on a rule id outside `registry`/generated code (ENG-11).
8. ADR files are numbered sequentially and have the four required headings.
9. Crate-level docs present and non-empty for each crate.

`cargo xtask ci` = `xtask check` + fmt + clippy + test + deny + machete + doc + coverage gate. Contributors run `cargo xtask ci` before pushing; CI runs the same command (single source of truth for "green").

**Deliberate-violation tests:** `xtask check` has its own tests that create a temp workspace with a forbidden edge, a `utils.rs`, and an untracked `TODO` and assert that each is rejected. A checker that cannot fail is not a checker.

### 12.12 Anti-pattern blacklist (reject in review; cite the ID)

1. God modules or files > 800 lines (ENG-10). 2. `utils`/`helpers`/`common` dumping grounds (ENG-16). 3. Boolean flag parameters (ENG-22). 4. Stringly-typed rule IDs, tags or kinds passed around internally (ENG-23). 5. Global mutable state or hidden singletons (ENG-05). 6. `unwrap()`/`expect()`/`panic!` in library code (ENG-25). 7. Copy-pasted rule files differing by a constant — extract data, not code (ENG-15). 8. A trait with one implementation and no I/O/testing/extension reason (ENG-20). 9. `.clone()` sprinkled to silence the borrow checker, without a comment (ENG-28). 10. `Rc<RefCell<_>>`/`Arc<Mutex<_>>` without an ADR (ENG-28). 11. I/O inside pure crates (ENG-02). 12. Speculative configurability / feature flags nobody asked for (ENG-20, ENG-43). 13. Macro-generated public API (ENG-18). 14. Behavior change hidden inside a "refactor" commit (ENG-44). 15. Output/message changes without snapshot updates and CHANGELOG (ENG-30, ENG-41). 16. False positives accepted "for now" — a false positive is a P0 bug (Section 0, Phase 7).

### 12.13 Contributor experience (open-source readiness)

These are *product requirements* for the repository, tracked in Phase 0 and verified in Phase 4:
- **One-command setup and check:** `cargo xtask setup` (verifies toolchain, installs optional git hooks: `fmt` + fast `clippy`), `cargo xtask check`, `cargo xtask ci`. A devcontainer/`mise`/`rust-toolchain.toml` makes a fresh clone build without reading docs.
- **Docs for newcomers:** `CONTRIBUTING.md` (setup → first PR walkthrough → "add a rule in 15 minutes" tutorial with a real, tiny rule), `docs/ARCHITECTURE.md` (30-minute architecture tour with a diagram of Section 4.1 and the crate responsibilities), `docs/ENGINEERING.md` (this section), `docs/adr/` (why things are the way they are).
- **Templates:** issue templates (bug with minimal repro, false-positive report, rule request, feature), PR template (below), discussion categories.
- **Labels & funnel:** `good first issue` (each with file pointers and an acceptance checklist), `help wanted`, `rule:<id>`, `area:<crate>`, `needs-repro`, `false-positive`. Parity-table issues are auto-generated (Phase 4).
- **Service levels:** first response ≤ 48 h; review within 5 working days; stale issues are closed *politely* after 60 days with an invitation to reopen.
- **Recognition:** contributors credited in release notes (`all-contributors` or changelog credits).
- **No bikeshedding:** style is automated; design debates happen in issues/ADRs, not in PR line comments.
- **Security & conduct:** `SECURITY.md` (private reporting channel), `CODE_OF_CONDUCT.md`, maintainers file.

### 12.14 Changing these rules
A rule is changed or waived only through an ADR approved by the maintainers. Waivers are scoped (one crate/file) and carry an expiry or revisit date. Repeated waivers of the same rule mean the rule is wrong — fix the rule.

---
