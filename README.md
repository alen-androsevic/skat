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

## Develop

```bash
cargo test
cargo run -- --help
```
