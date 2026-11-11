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
  skat split      turn a finished skill into parts in dev/skill/

Each directory in dev/skill/ whose name ends in .md becomes that file: its .md
parts, in name order, joined by one blank line. Other files are copied as they
are. <name> is the name in the SKILL.md frontmatter. Files in skills/<name>/
that the build doesn't produce are removed.

Run it anywhere inside the skill's repository; it finds dev/skill/ in the
working directory or above it.

skat split [DIR] reads the finished skill in DIR, by default the one skill in
skills/, and writes its parts to dev/skill/ in the working directory. It cuts
each .md file at its frontmatter and at every # and ## heading, and copies the
other files. A .md file that wouldn't rebuild byte for byte, such as one with
two blank lines before a heading, is copied whole. It stops if dev/skill/
exists.

Options:
  --check      Change nothing; exit 1 if a build would change something
  --dry-run    Change nothing; list what a build or split would write
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "split") {
        return split(&args[1..]);
    }
    let mut opts = Opts::default();
    for arg in args {
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

/// `skat split`: writes the parts of a finished skill to `dev/skill/` in the working directory.
fn split(args: &[String]) -> Result<ExitCode, String> {
    let mut opts = Opts::default();
    let mut from = None;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(ExitCode::SUCCESS);
            }
            "--dry-run" => opts.dry_run = true,
            "--json" => opts.json = true,
            _ if arg.starts_with('-') => {
                return Err(format!(
                    "unknown option {arg} for split. Run `skat --help`."
                ));
            }
            _ if from.is_some() => {
                return Err("split takes one skill directory. Run `skat --help`.".to_string());
            }
            _ => from = Some(PathBuf::from(arg)),
        }
    }
    let from = match from {
        Some(dir) => dir,
        None => only_skill()?,
    };
    if !from.join("SKILL.md").is_file() {
        return Err(format!(
            "{}: no SKILL.md. Pass the directory of a finished skill.",
            from.display()
        ));
    }
    let to = Path::new(SRC);
    if to.exists() {
        return Err(format!(
            "{SRC}/ already exists. Edit its parts and run `skat`, or remove it to split again."
        ));
    }

    let source = existing_files(&from)?;
    let mut planned = BTreeMap::new();
    let (mut split_files, mut copied) = (Vec::new(), Vec::new());
    for (rel, bytes) in &source {
        let is_md = rel.extension().is_some_and(|ext| ext == "md");
        let parts = is_md
            .then(|| std::str::from_utf8(bytes).ok().and_then(cut))
            .flatten();
        match parts {
            Some(parts) => {
                for (name, text) in parts {
                    planned.insert(rel.join(name), text.into_bytes());
                }
                split_files.push(rel.clone());
            }
            None => {
                planned.insert(rel.clone(), bytes.clone());
                copied.push(rel.clone());
            }
        }
    }

    if !opts.dry_run {
        for (rel, bytes) in &planned {
            let path = to.join(rel);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir).map_err(|e| io_err(dir, e))?;
            }
            fs::write(&path, bytes).map_err(|e| io_err(&path, e))?;
        }
        // The parts must build back into the files they came from.
        let mut build = Build::default();
        collect(to, Path::new(""), &mut build)?;
        if build.files != source {
            return Err(format!(
                "the parts in {SRC}/ don't rebuild {} exactly. Remove {SRC}/ and report this as a skat bug.",
                from.display()
            ));
        }
    }

    let paths = |dir: &Path, rels: &[PathBuf]| -> Vec<String> {
        rels.iter()
            .map(|rel| dir.join(rel).display().to_string())
            .collect()
    };
    let written = paths(to, &planned.keys().cloned().collect::<Vec<_>>());
    if opts.json {
        let list = |items: Vec<String>| {
            let items: Vec<String> = items.iter().map(|s| json_str(s)).collect();
            format!("[{}]", items.join(","))
        };
        println!(
            "{{\"from\":{},\"to\":{},\"written\":{},\"split\":{},\"copied\":{}}}",
            json_str(&from.display().to_string()),
            json_str(SRC),
            list(written),
            list(paths(&from, &split_files)),
            list(paths(&from, &copied))
        );
    } else {
        let (verb, summary) = if opts.dry_run {
            ("would write", "Would split")
        } else {
            ("wrote", "Split")
        };
        for path in &written {
            println!("{verb}  {path}");
        }
        let count = |n: usize, word: &str| match n {
            1 => format!("1 {word}"),
            n => format!("{n} {word}s"),
        };
        println!(
            "{summary} {} into {SRC}/: {} in {}, {} copied as they are.",
            from.display(),
            count(split_files.len(), "file"),
            count(planned.len() - copied.len(), "part"),
            copied.len()
        );
        if !opts.dry_run {
            println!("Edit the parts, then run `skat`.");
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The one skill directory in `skills/`, for `skat split` without a directory.
fn only_skill() -> Result<PathBuf, String> {
    let skills = Path::new("skills");
    if !skills.is_dir() {
        return Err("no skills/ here. Pass the skill directory: `skat split DIR`.".to_string());
    }
    let dirs: Vec<PathBuf> = sorted_entries(skills)?
        .into_iter()
        .filter(|entry| entry.is_dir())
        .collect();
    match dirs.as_slice() {
        [dir] => Ok(dir.clone()),
        [] => Err(
            "skills/ has no skill in it. Pass the skill directory: `skat split DIR`.".to_string(),
        ),
        _ => Err(format!(
            "skills/ has {} skills. Pick one: `skat split skills/NAME`.",
            dirs.len()
        )),
    }
}

/// Cuts Markdown into named parts at its frontmatter and at each `#` and `##` heading outside
/// code blocks. `None` when that gives one part, or when the parts wouldn't join back into `text`.
fn cut(text: &str) -> Option<Vec<(String, String)>> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut starts: Vec<(usize, String)> = Vec::new();
    let mut body = 0;
    if lines.first().is_some_and(|line| line.trim_end() == "---")
        && let Some(end) = lines[1..].iter().position(|line| line.trim_end() == "---")
    {
        starts.push((0, "frontmatter".to_string()));
        body = end + 2;
    }
    let mut in_fence = false;
    for (n, line) in lines.iter().enumerate().skip(body) {
        let fence = line.trim_start().starts_with("```");
        let title = match in_fence || fence {
            true => None,
            false => line.strip_prefix("# ").or_else(|| line.strip_prefix("## ")),
        };
        if let Some(title) = title {
            starts.push((n, slug(title)));
        } else if starts.last().is_none_or(|(start, _)| *start < body) && !line.trim().is_empty() {
            starts.push((n, "intro".to_string()));
        }
        if fence {
            in_fence = !in_fence;
        }
    }
    if starts.len() < 2 {
        return None;
    }
    starts[0].0 = 0;

    let width = ((starts.len() - 1) * 10).to_string().len().max(2);
    let mut texts = Vec::new();
    let mut parts = Vec::new();
    for (i, (start, name)) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map_or(lines.len(), |(next, _)| *next);
        let part = lines[*start..end].concat();
        let part = part.trim_start_matches(['\n', '\r']).trim_end().to_string();
        parts.push((format!("{:0width$}-{name}.md", i * 10), format!("{part}\n")));
        texts.push(part);
    }
    (join(&texts) == text).then_some(parts)
}

/// A heading as a file name: lowercase ASCII letters and digits joined by hyphens, cut after the
/// last whole word that fits in 32 characters. A step number such as `2.` is left out, since the
/// part's own number already orders it.
fn slug(title: &str) -> String {
    let title = title.trim().trim_end_matches('#');
    let unnumbered = title
        .split_once(". ")
        .filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        .map_or(title, |(_, rest)| rest);
    let mut s = String::new();
    for c in unnumbered.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    if s.len() > 32 {
        let end = s[..=32].rfind('-').unwrap_or(32);
        s.truncate(end);
    }
    let s = s.trim_end_matches('-');
    if s.is_empty() {
        "section".to_string()
    } else {
        s.to_string()
    }
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
    fn cut_splits_at_frontmatter_and_top_headings() {
        let text = "---\nname: demo\n---\n\nIntro.\n\n# Demo\n\n```\n# not a heading\n```\n\n\
                    ### Kept inside\n\n## Steps & more\n\nDo it.\n";
        let parts = cut(text).unwrap();
        let names: Vec<&str> = parts.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "00-frontmatter.md",
                "10-intro.md",
                "20-demo.md",
                "30-steps-more.md"
            ]
        );
        assert_eq!(parts[0].1, "---\nname: demo\n---\n");
        assert!(parts[2].1.contains("# not a heading") && parts[2].1.contains("### Kept"));
        let texts: Vec<String> = parts
            .iter()
            .map(|(_, t)| t.trim_end().to_string())
            .collect();
        assert_eq!(join(&texts), text);
    }

    #[test]
    fn cut_keeps_whole_what_wouldnt_rebuild() {
        assert_eq!(cut("# One section\n\ntext\n"), None);
        assert_eq!(cut("# A\n\n\n## B\n"), None);
        assert_eq!(cut("# A\n\n## B"), None);
    }

    #[test]
    fn cut_widens_numbers_past_ninety() {
        let text: Vec<String> = (0..11).map(|i| format!("## S{i}")).collect();
        let parts = cut(&join(&text)).unwrap();
        assert_eq!(parts[0].0, "000-s0.md");
        assert_eq!(parts[10].0, "100-s10.md");
    }

    #[test]
    fn slug_makes_safe_file_names() {
        assert_eq!(slug("1. Find the range"), "find-the-range");
        assert_eq!(slug("2024. A year"), "a-year");
        assert_eq!(slug("v2. Upgrade"), "v2-upgrade");
        assert_eq!(slug("2. "), "2");
        assert_eq!(
            slug("Load the driver: web & mobile ##\n"),
            "load-the-driver-web-mobile"
        );
        assert_eq!(slug("¿?"), "section");
        assert_eq!(
            slug("Mobile driver (adb Android, simctl and computer use)"),
            "mobile-driver-adb-android-simctl"
        );
        assert_eq!(slug(&"x".repeat(40)), "x".repeat(32));
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
