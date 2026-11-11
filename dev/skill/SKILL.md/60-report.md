## 6. Report

Show the parts as a tree, then the next step:

```
dev/skill/SKILL.md/00-frontmatter.md
dev/skill/SKILL.md/10-<first-section>.md
...
```

Next: edit the parts in `dev/skill/`, run `skat`, and commit `dev/skill/` and `skills/<name>/` together. Don't commit, push or publish for the user.

Later changes go in `dev/skill/`, because `skat` overwrites `skills/<name>/`. If another tool edits `skills/<name>/` directly, run `skat --check` first: when it exits 0, nothing in `dev/skill/` is lost by removing it and running `skat split` again.
