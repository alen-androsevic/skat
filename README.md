<img alt="" src="assets/icon.svg" width="96">

# skat

skat (skill + cat) builds a Claude Code skill from parts, so you can work on one section of a SKILL.md or reference file at a time.

## Install

```bash
cargo install --git https://github.com/alen-androsevic/skat
```

One binary with no dependencies, for macOS and Linux. It lands in `~/.cargo/bin`, so put that on your `PATH`. `cargo uninstall skat` removes it.

## Use

Inside a skill's repository:

```bash
skat            # build skills/<name>/ from dev/skill/
skat --check    # exit 1 if skills/<name>/ is out of date
skat --dry-run  # show what a build would change
skat split      # turn a finished skill into parts in dev/skill/
```

Add `--json` for machine-readable output. Exit codes: 0 built or up to date, 1 out of date (`--check`), 2 error.

## Layout

```
dev/skill/                        the parts: edit these
  SKILL.md/
    00-frontmatter.md
    10-scope.md
    20-attack.md
  references/
    web.md/
      00-intro.md
      10-dialog-guard.md
  scripts/seed.js                 copied as it is
skills/<name>/                    built: don't edit by hand
  SKILL.md
  references/web.md
  scripts/seed.js
```

- Each directory whose name ends in `.md` becomes one file. Its `.md` parts are joined in name order with one blank line between them. Number them in steps of ten, so a new part fits between two others without renaming anything.
- Other files are copied as they are. Dotfiles such as `.DS_Store` are skipped.
- `<name>` is the `name:` in the SKILL.md frontmatter.
- A build removes files from `skills/<name>/` that no part produces.
- A build stops, naming the part and line, on:
  - a relative link to a file the build doesn't produce;
  - a file in a parts directory that isn't `.md`;
  - a frontmatter `name` or `description` that Claude Code would reject.

Commit the parts and the built files together. Plugins and plain skill installs read `skills/<name>/`.

## Make a new skill

The skat plugin asks a few multiple-choice questions, writes a first draft with Claude's skill creator, and splits it into parts. Install it from the [alen-androsevic marketplace](https://github.com/alen-androsevic/skills):

```bash
claude plugin marketplace add alen-androsevic/skills
claude plugin install skat@alen-androsevic
```

Then, in a new repository, run `/skat:new-skill` or ask Claude Code to make a skill. It needs the `skat` binary on your `PATH`.

Without the plugin, write the skill to `skills/<name>/` with `/skill-creator` or by hand, then split it:

```bash
skat split      # cut skills/<name>/ into parts in dev/skill/
skat --check    # exit 0: the parts rebuild it exactly
```

`skat split` cuts each `.md` file at its frontmatter and at every `#` and `##` heading, and copies the other files. A `.md` file whose blank lines wouldn't rebuild exactly stays one part. It takes the skill directory as an argument when `skills/` holds more than one, and stops if `dev/skill/` exists.

## Develop

```bash
cargo test
cargo run -- --help
```

This repository is also the skat plugin. Its skill, `new-skill`, is built with skat itself: edit `dev/skill/`, then run `cargo run`.
