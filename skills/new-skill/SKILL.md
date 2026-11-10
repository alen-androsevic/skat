---
name: new-skill
description: Create a new Claude Code skill through a few multiple-choice questions, written as skat parts in dev/skill/ so each section can be edited on its own. Use when the user wants to create, start, make or scaffold a skill or plugin skill, or says /new-skill. Not for changing an existing skill; edit its parts in dev/skill/ and run skat.
---

# New skill

Makes a new skill in the working repository: a few questions, a first draft, then `skat split` turns the draft into parts in `dev/skill/`. The finished skill is built into `skills/<name>/`.

## 1. Check first

Run these in order. At the first one that fails, do what it says, and go on only if it says so:

- `command -v skat` finds nothing: tell the user to install it with `cargo install --git https://github.com/alen-androsevic/skat`, and stop.
- `git rev-parse --show-toplevel` fails: the folder isn't a git repository. Ask whether to run `git init` here; go on if the user says yes, and stop if not. Otherwise run the rest from the folder it prints.
- `dev/skill/` exists: the repository already has a skill, and skat builds one skill per repository. Tell the user to edit its parts and run `skat`, or to start in a new repository, and stop.
- `skills/` holds another skill: say which, and ask whether to go on in a new repository instead.

## 2. Ask

If the request doesn't say what the skill does and when it should load, ask that first, in one plain question. That answer is free text, so it doesn't go in a multiple-choice question.

Then ask the rest in one AskUserQuestion call. Leave out a question the request already answers, and put the recommended option first.

| Header | Question | Options |
|---|---|---|
| Name | What should the skill be called? | Two or three names you suggest: lowercase letters, digits and hyphens, at most 64 characters, named for the job |
| Loading | When should Claude use it? | Automatically when a task matches (recommended); only when typed as `/<name>` |
| Files | What does it need besides SKILL.md? (multiSelect) | Nothing else (recommended for a first draft); reference files Claude reads when needed; scripts for steps that must come out the same every time; assets such as templates |
| Tests | Test it before you stop? | No, write the first draft, about 5 minutes (recommended); yes, run the skill creator's evals, 30 minutes or more |

"Only when typed" adds `disable-model-invocation: true` to the frontmatter.

## 3. Write the draft

Write the finished skill to `skills/<name>/`, as one `SKILL.md` plus the files the user chose.

- If the skill creator is installed (`skill-creator` in the skill list, with or without a plugin prefix), use it, and pass on the answers. Tell it where things go: the skill in `skills/<name>/`, its workspace in `dev/<name>-workspace/`, never inside `skills/`. Without tests, the skill creator stops after the first draft.
- Otherwise write it yourself: the frontmatter with `name` and `description`, then the steps. The description says what the skill does and when to use it, in the words a user would type, and when not to.
- With tests, move `skills/<name>/evals/` to `dev/evals/` when the skill creator is done, so the evals don't ship with the skill.

Give each section of SKILL.md and of each reference file a `#` or `##` heading, with one blank line before it. skat cuts the parts there.

## 4. Split it into parts

```bash
skat split skills/<name>
skat --check
```

`skat split` writes `dev/skill/` and checks that the parts rebuild the draft byte for byte. `skat --check` exits 0 when `skills/<name>/` is up to date. On any other result, show the output and stop.

A `.md` file listed as copied, rather than split, either has one section or blank lines skat can't rebuild exactly. It still builds; it just stays one part.

## 5. Report

Show the parts as a tree, then the next step:

```
dev/skill/SKILL.md/00-frontmatter.md
dev/skill/SKILL.md/10-<first-section>.md
...
```

Next: edit the parts in `dev/skill/`, run `skat`, and commit `dev/skill/` and `skills/<name>/` together. Don't commit, push or publish for the user.

Later changes go in `dev/skill/`, because `skat` overwrites `skills/<name>/`. If the skill creator edits `skills/<name>/` again, run `skat --check` first: when it exits 0, nothing in `dev/skill/` is lost by removing it and running `skat split` again.
