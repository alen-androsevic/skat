# skat

A CLI that builds `skills/<name>/` from the parts in `dev/skill/` of a Claude Code skill repository. See README.md for the layout rules.

- The repository is also the skat plugin. Its skill, `new-skill`, is in `dev/skill/`; build it with `cargo run`.
- One file, `src/main.rs`, std only. Keep it that way unless a dependency removes a lot of code.
- macOS and Linux only.
- Data and results on stdout, errors on stderr prefixed `skat:`. Exit 0 on success, 1 when `--check` finds drift, 2 on any error.
- The `--json` shape is stable. Changing it needs a changelog entry.
- Check changes with `cargo test`, `cargo clippy` and `cargo fmt`. Then run `cargo run --manifest-path PATH/TO/skat/Cargo.toml -- --check` inside a clone of [break-stuff](https://github.com/alen-androsevic/break-stuff), which is built with skat and must stay up to date.

## MIG

Skin: alen (terminal output only; it uses no colour)
