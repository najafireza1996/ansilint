# 0004. Solo-maintainer git workflow without pull requests

Status: accepted

## Context

ENG-46 asks for at least one approving review and branch protection, which
assumes several maintainers. The project has one maintainer, who works with an
AI agent: a pull request would be reviewed by its own author, and the old
"self-merge after a 24 h wait" rule only delayed work without adding a second
pair of eyes. Spec v1.2 adds Section 12.9a, a workflow without pull requests,
and asks for this ADR (ENG-40, 12.14).

Two facts shaped the decision (checked 2026-11-04):

- `.github/workflows/ci.yml` ran on push only for `main`, so a branch got no
  CI before it was merged. It now runs on pushes to every branch, and
  `commit-lint` checks `origin/main..HEAD` on branch pushes.
- `main` has no branch protection and no rulesets
  (`gh api repos/najafireza1996/ansilint/branches/main/protection` returns 404).

## Decision

While the project has a single maintainer, the maintainer (and an AI agent
acting for them) follows Section 12.9a of the spec, copied into
`docs/ENGINEERING.md`:

1. `main` is always green and releasable; its history only grows.
2. Every change lives on a short-lived branch cut from `main`
   (`feat/`, `fix/`, `refactor/`, `chore/`, `docs/`), one concern per branch.
3. Conventional Commits, one per task, each passing `cargo xtask ci`.
4. Merge gate, checked rather than assumed: local `cargo xtask ci` and the MSRV
   all-targets check are green; GitHub Actions is green on Linux, macOS and
   Windows for the exact head commit; docs, `CHANGELOG.md` and the phase report
   are updated; no open P0/P1; the phase's last merge has the independent
   review recorded (otherwise it may merge but not be tagged); no check was
   weakened without an ADR.
5. `git merge --no-ff <branch>`, message `merge(<scope>): <summary>`, push
   `main`, then delete the branch with `git branch -d` locally and remotely.
6. Stop-the-line: if CI on `main` turns red, nothing else merges until it is
   fixed forward or the merge is reverted (`git revert -m 1`).
7. Annotated tags only on `main` and only after the phase gate; never move or
   delete a pushed tag.
8. Never force-push, rewrite published history, merge on red or unknown CI,
   use `--no-verify`, weaken a check, or `git branch -D` unmerged work.

External contributors keep the fork and pull-request flow in `CONTRIBUTING.md`;
ENG-46 still applies to their pull requests.

## Consequences

- Work merges as soon as the three-OS CI is green; the 24 h wait is gone.
- **No server-side enforcement of status checks:** GitHub cannot require
  status checks for direct pushes to `main`, so the merge gate and the
  stop-the-line rule rely on discipline. Mitigation: CI also runs on `main`,
  and the maintainer should add a repository rule that blocks force-pushes to
  and deletion of `main` (not configured as of this ADR).
- `--no-ff` merges keep per-task commits visible but make history non-linear;
  "require linear history" must stay off for `main`.
- No second reviewer sees the code before it lands; the independent review
  session (spec Appendix A.4) per phase is the substitute.

## Alternatives considered

- **Pull-request mode (ENG-46 as written):** the maintainer opens a pull
  request and self-approves after 24 h. Rejected while there is one maintainer:
  the review is a formality and the wait slows every change. Switch back, with
  a superseding ADR, when a second maintainer or regular external contributors
  appear.
- **Committing straight to `main`:** no branch CI before a change lands, so a
  red `main` would be found only after the fact. Rejected.
