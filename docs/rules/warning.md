# warning

- **Profile:** min
- **Tags:** warning
- **Fixable:** no

The upstream `warning` meta-rule for non-fatal notices. In Phase 1 it reports an
empty playbook: a playbook whose plays contain no tasks, handlers or roles.

## Examples

Fails:

```yaml
- hosts: all
```

Passes:

```yaml
- hosts: all
  tasks:
    - name: say hello
      ansible.builtin.debug:
        msg: hello
```

## Skipping

Warnings are non-fatal by default. They can be silenced with `# noqa: warning`,
hidden with `skip_list`, or turned into failures with `strict`. The exact
upstream message text is verified before the first release (OPEN_QUESTIONS #7).
