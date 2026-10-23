## What and why

<!-- One paragraph. Link the issue. -->

## Type

- [ ] Rule
- [ ] Fix
- [ ] Feature
- [ ] Bug fix
- [ ] Refactor (no behavior change)
- [ ] Docs/CI

## Checklist

- [ ] `cargo xtask ci` passes locally
- [ ] Tests added first for bugs; fixtures added for rules (fail / pass / noqa / fix)
- [ ] No new `unwrap`/`expect`/`panic`/`unsafe`; errors are typed
- [ ] No I/O added to a pure crate; no new crate edge (or ADR linked: ____ )
- [ ] New dependency? Justification answered below (ENG-42)
- [ ] User-visible change: CHANGELOG + docs updated; snapshots reviewed
- [ ] Rule docs page + `parity.toml` updated (if a rule changed)
- [ ] Benchmarks attached if this touches a hot path
- [ ] Any new TODO is `TODO(#issue)` with a `docs/TECH_DEBT.md` entry

## Dependency justification (if applicable)

what / why not std or existing deps / license / maintenance / MSRV / unsafe / size & compile-time impact

## Reviewer notes

<!-- Cite ENG-xx IDs where a rule shaped the decision. Output/message changes are public API. -->
