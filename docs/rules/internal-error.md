# internal-error

- **Profile:** min
- **Tags:** internal
- **Fixable:** no

A rule panicked or failed internally. The run continues: the failing rule is
isolated and the finding names the rule so it can be reported.

## What triggers it

A panic inside a rule implementation. This is always a bug in `ansilint`, not in
the input, and should be reported with a minimal reproduction.

## Examples

The runner catches the panic and emits:

```
file.yml:1: internal-error: rule `some-rule` panicked: ...
```

## Skipping

Not skippable. Fix the rule instead.
