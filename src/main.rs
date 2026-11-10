//! Builds `skills/<name>/` from the parts in `dev/skill/`.
//!
//! A directory in `dev/skill/` whose name ends in `.md` is one output file: its `.md` parts, in
//! name order, joined by one blank line. Every other file is copied as it is. The skill's name,
//! and so the output directory, comes from the `name:` line in the SKILL.md frontmatter.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "\
Builds a Claude Code skill in skills/<name>/ from the parts in dev/skill/.

Examples:
  skat            build the skill
  skat --check    exit 1 if skills/<name>/ is out of date
  skat --dry-run  show what a build would change

Each directory in dev/skill/ whose name ends in .md becomes that file: its .md
parts, in name order, joined by one blank line. Other files are copied as they
are. <name> is the name in the SKILL.md frontmatter. Files in skills/<name>/
that the build doesn't produce are removed.

Run it anywhere inside the skill's repository; it finds dev/skill/ in the
working directory or above it.

Options:
  --check      Change nothing; exit 1 if a build would change something
  --dry-run    Change nothing; list what a build would write and remove
  --json       Print the result as JSON on stdout
  -h, --help   Show this help

Exit codes: 0 built or up to date, 1 out of date (--check), 2 error.
";

const SRC: &str = "dev/skill";

#[derive(Default)]
struct Opts {
    check: bool,
    dry_run: bool,
    json: bool,
}

/// One piece of text that ends up in an output file, kept for error messages.
struct Part {
    src: PathBuf,
    out: PathBuf,
    text: String,
}

#[derive(Default)]
struct Build {
    files: BTreeMap<PathBuf, Vec<u8>>,
    parts: Vec<Part>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("skat: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut opts = Opts::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(ExitCode::SUCCESS);
            }
            "--check" => opts.check = true,
            "--dry-run" => opts.dry_run = true,
            "--json" => opts.json = true,
            _ => return Err(format!("unknown option {arg}. Run `skat --help`.")),
        }
    }

    // Work from the repository root, so every path in a message is relative to it.
    let root = find_root()?;
    std::env::set_current_dir(&root).map_err(|e| io_err(&root, e))?;

    let mut build = Build::default();
    collect(Path::new(SRC), Path::new(""), &mut build)?;
    let name = skill_name(&build)?;
    check_links(&build)?;

    let out = Path::new("skills").join(&name);
    let existing = existing_files(&out)?;

    let mut written = Vec::new();
    let mut unchanged = 0;
    for (rel, bytes) in &build.files {
        match existing.get(rel) {
            Some(old) if old == bytes => unchanged += 1,
            _ => written.push(rel.clone()),
        }
    }
    let removed: Vec<PathBuf> = existing
        .keys()
        .filter(|rel| !build.files.contains_key(*rel))
        .cloned()
        .collect();
    let up_to_date = written.is_empty() && removed.is_empty();

    if !opts.check && !opts.dry_run {
        for rel in &written {
            let path = out.join(rel);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
            }
            fs::write(&path, &build.files[rel]).map_err(|e| io_err(&path, e))?;
        }
        for rel in &removed {
            let path = out.join(rel);
            fs::remove_file(&path).map_err(|e| io_err(&path, e))?;
            remove_empty_parents(&out, &path);
        }
    }

    let shown = |rel: &PathBuf| out.join(rel).display().to_string();
    if opts.json {
        let list = |paths: &[PathBuf]| {
            let items: Vec<String> = paths.iter().map(|p| json_str(&shown(p))).collect();
            format!("[{}]", items.join(","))
        };
        println!(
            "{{\"out\":{},\"up_to_date\":{},\"written\":{},\"removed\":{},\"unchanged\":{}}}",
            json_str(&out.display().to_string()),
            up_to_date,
            list(&written),
            list(&removed),
            unchanged
        );
    } else {
        let (write_word, remove_word) = match (opts.check, opts.dry_run) {
            (true, _) => ("differs", "not in dev/skill"),
            (false, true) => ("would write", "would remove"),
            _ => ("wrote", "removed"),
        };
        for rel in &written {
            println!("{write_word}  {}", shown(rel));
        }
        for rel in &removed {
            println!("{remove_word}  {}", shown(rel));
        }
        let out_shown = out.display();
        if up_to_date {
            println!("{out_shown} is up to date.");
        } else if opts.check {
            println!("{out_shown} is out of date. Run `skat`.");
        } else {
            let verb = if opts.dry_run { "Would build" } else { "Built" };
            println!(
                "{verb} {out_shown}: {} written, {} removed, {unchanged} unchanged.",
                written.len(),
                removed.len()
            );
        }
    }

    Ok(if opts.check && !up_to_date {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// The nearest directory, from the working directory up, that holds `dev/skill/`.
fn find_root() -> Result<PathBuf, String> {
    let cwd =
        std::env::current_dir().map_err(|e| format!("can't read the working directory: {e}"))?;
    cwd.ancestors()
        .find(|dir| dir.join(SRC).is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "no {SRC}/ in {} or above it. Run it inside a skill repository that has {SRC}/.",
                cwd.display()
            )
        })
}

/// Walks `dir` (at `rel` inside the skill) and adds every output file to `build`.
fn collect(dir: &Path, rel: &Path, build: &mut Build) -> Result<(), String> {
    for entry in sorted_entries(dir)? {
        let name = entry.file_name().unwrap().to_string_lossy().into_owned();
        let out = rel.join(&name);
        if entry.is_dir() && name.ends_with(".md") {
            let text = assemble(&entry, &out, build)?;
            build.files.insert(out, text.into_bytes());
        } else if entry.is_dir() {
            collect(&entry, &out, build)?;
        } else {
            let bytes = fs::read(&entry).map_err(|e| io_err(&entry, e))?;
            if name.ends_with(".md") {
                let text = String::from_utf8(bytes.clone())
                    .map_err(|_| format!("{}: not UTF-8", entry.display()))?;
                build.parts.push(Part {
                    src: entry.clone(),
                    out: out.clone(),
                    text,
                });
            }
            build.files.insert(out, bytes);
        }
    }
    Ok(())
}

/// Joins the parts in `dir` into the text of the output file `out`.
fn assemble(dir: &Path, out: &Path, build: &mut Build) -> Result<String, String> {
    let mut texts = Vec::new();
    for entry in sorted_entries(dir)? {
        if entry.is_dir() || entry.extension().is_none_or(|ext| ext != "md") {
            return Err(format!(
                "{}: parts of {} must be .md files. Move it out, or rename it to end in .md.",
                entry.display(),
                out.display()
            ));
        }
        let text = fs::read_to_string(&entry).map_err(|e| io_err(&entry, e))?;
        let text = text.trim_start_matches(['\n', '\r']).trim_end().to_string();
        if !text.is_empty() {
            texts.push(text.clone());
        }
        build.parts.push(Part {
            src: entry,
            out: out.to_path_buf(),
            text,
        });
    }
    if texts.is_empty() {
        return Err(format!(
            "{}: no parts with text in it. Add a .md part, or remove the directory.",
            dir.display()
        ));
    }
    Ok(join(&texts))
}

fn join(texts: &[String]) -> String {
    let mut s = texts.join("\n\n");
    s.push('\n');
    s
}

/// Entries of `dir` by name, without dotfiles such as `.DS_Store`.
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err(dir, e))? {
        let entry = entry.map_err(|e| io_err(dir, e))?;
        if !entry.file_name().to_string_lossy().starts_with('.') {
            entries.push(entry.path());
        }
    }
    entries.sort();
    Ok(entries)
}

/// The `name:` from the SKILL.md frontmatter, checked against the rules for skill names.
fn skill_name(build: &Build) -> Result<String, String> {
    let skill = build.files.get(Path::new("SKILL.md")).ok_or(format!(
        "no SKILL.md. Add {SRC}/SKILL.md/ with a frontmatter part."
    ))?;
    let text = String::from_utf8_lossy(skill);
    let front = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(front, _)| front)
        .ok_or("SKILL.md doesn't start with frontmatter between two `---` lines.")?;

    let field = |key: &str| {
        front.lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
            Some(value.trim_matches(['"', '\'']).to_string())
        })
    };
    let name = field("name").ok_or("the SKILL.md frontmatter has no `name:`.")?;
    let description =
        field("description").ok_or("the SKILL.md frontmatter has no `description:`.")?;

    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    if name.is_empty() || name.len() > 64 || !name.chars().all(valid) {
        return Err(format!(
            "skill name `{name}` must be 1 to 64 lowercase letters, digits and hyphens."
        ));
    }
    if description.is_empty() || description.chars().count() > 1024 {
        return Err(format!(
            "the description is {} characters; it must be 1 to 1024.",
            description.chars().count()
        ));
    }
    Ok(name)
}

/// Fails on a relative Markdown link to a file the build doesn't produce.
fn check_links(build: &Build) -> Result<(), String> {
    let mut broken = Vec::new();
    for part in &build.parts {
        let base = part.out.parent().unwrap_or(Path::new(""));
        for (line_no, target) in links(&part.text) {
            let found = normalize(&base.join(target)).is_some_and(|p| build.files.contains_key(&p));
            if !found {
                broken.push(format!("  {}:{line_no}: {target}", part.src.display()));
            }
        }
    }
    if broken.is_empty() {
        return Ok(());
    }
    let count = match broken.len() {
        1 => "1 link points".to_string(),
        n => format!("{n} links point"),
    };
    Err(format!(
        "{count} to a file the build doesn't produce:\n{}",
        broken.join("\n")
    ))
}

/// Relative link targets in `text`, with their line numbers, outside code blocks and code spans.
fn links(text: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for (j, segment) in line.split('`').enumerate() {
            if j % 2 == 1 {
                continue;
            }
            let mut rest = segment;
            while let Some(start) = rest.find("](") {
                rest = &rest[start + 2..];
                let Some(end) = rest.find(')') else { break };
                let target = rest[..end].split_whitespace().next().unwrap_or("");
                let target = target.split('#').next().unwrap_or("");
                let external = target.contains("://") || target.starts_with("mailto:");
                if !target.is_empty() && !external {
                    found.push((i + 1, target));
                }
                rest = &rest[end..];
            }
        }
    }
    found
}

/// Resolves `.` and `..` in a relative path; `None` when it climbs out of the skill.
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

/// Every file under `out`, by path relative to it. Dotfiles are left alone.
fn existing_files(out: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    if !out.is_dir() {
        return Ok(files);
    }
    let mut stack = vec![out.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in sorted_entries(&dir)? {
            if entry.is_dir() {
                stack.push(entry);
            } else {
                let bytes = fs::read(&entry).map_err(|e| io_err(&entry, e))?;
                files.insert(entry.strip_prefix(out).unwrap().to_path_buf(), bytes);
            }
        }
    }
    Ok(files)
}

/// Removes the directories between `file` and `out` that the removal left empty.
fn remove_empty_parents(out: &Path, file: &Path) {
    for dir in file.ancestors().skip(1) {
        if dir == out || fs::remove_dir(dir).is_err() {
            break;
        }
    }
}

fn io_err(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

fn json_str(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_puts_one_blank_line_between_parts() {
        let texts = vec!["# A".to_string(), "## B\ntext".to_string()];
        assert_eq!(join(&texts), "# A\n\n## B\ntext\n");
    }

    #[test]
    fn links_skips_code_external_and_anchors() {
        let text = "See [web](references/web.md#guard) and [site](https://x.dev).\n\
                    `[not](a.md)` [anchor](#top)\n\
                    ```\n[fenced](b.md)\n```\n\
                    [up](../SKILL.md)";
        assert_eq!(
            links(text),
            vec![(1, "references/web.md"), (6, "../SKILL.md")]
        );
    }

    #[test]
    fn normalize_refuses_to_leave_the_skill() {
        assert_eq!(
            normalize(Path::new("references/../SKILL.md")),
            Some(PathBuf::from("SKILL.md"))
        );
        assert_eq!(normalize(Path::new("references/../../README.md")), None);
    }
}
