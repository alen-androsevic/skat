## 1. Check first

Run these in order. At the first one that fails, do what it says, and go on only if it says so:

- `command -v skat` finds nothing: tell the user to install it with `cargo install --git https://github.com/alen-androsevic/skat`, and stop.
- `git rev-parse --show-toplevel` fails: the folder isn't a git repository. Ask whether to run `git init` here; go on if the user says yes, and stop if not. Otherwise run the rest from the folder it prints.
- `dev/skill/` exists: the repository already has a skill, and skat builds one skill per repository. Tell the user to edit its parts and run `skat`, or to start in a new repository, and stop.
- `skills/` holds another skill: say which, and ask whether to go on in a new repository instead.
