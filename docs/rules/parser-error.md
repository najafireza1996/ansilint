# parser-error

- **Profile:** min
- **Tags:** syntax
- **Fixable:** no

The YAML parser could not parse the file. The finding carries the scanner's byte
span so editors and the PEP8 formatter can point at the exact problem.

## What triggers it

Invalid YAML: an unclosed flow sequence, bad indentation of a block scalar, a
duplicate anchor definition, or an unterminated quoted scalar.

## Examples

Fails:

```yaml
a: [1,
```

Passes:

```yaml
a: 1
```

## Skipping

Parser errors are mandatory and cannot be silenced with an inline `# noqa`.
`warn_list` may downgrade them, and the ignore file may still hide them; both are
documented differences to verify against upstream (OPEN_QUESTIONS #11).
