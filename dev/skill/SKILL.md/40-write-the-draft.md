## 3. Write the draft

Write the finished skill to `skills/<name>/`, as one `SKILL.md` plus the files the user chose.

- If the skill creator is installed (`skill-creator` in the skill list, with or without a plugin prefix), use it, and pass on the answers. Tell it where things go: the skill in `skills/<name>/`, its workspace in `dev/<name>-workspace/`, never inside `skills/`. Without tests, the skill creator stops after the first draft.
- Otherwise write it yourself: the frontmatter with `name` and `description`, then the steps. The description says what the skill does and when to use it, in the words a user would type, and when not to.
- With tests, move `skills/<name>/evals/` to `dev/evals/` when the skill creator is done, so the evals don't ship with the skill.

Give each section of SKILL.md and of each reference file a `#` or `##` heading, with one blank line before it. skat cuts the parts there.
