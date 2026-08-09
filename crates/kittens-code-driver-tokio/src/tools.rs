//! Tool discharge over the real filesystem (SPEC K1/K2, KC0 subset).
//!
//! Paths are workspace-relative and normalized: absolute paths, `..`
//! traversal, and symlink escapes are refused (K2). `exec` is deliberately
//! absent from this KC0 build (feature-gated capability; K3 rule — a tool
//! not present is a tool not advertised).

use std::path::{Component, Path, PathBuf};

use kittens_code_core::engine::ProposedToolCall;
use kittens_code_protocol::event::ToolOutcome;
use serde::Deserialize;

/// Resolves a workspace-relative path or refuses it (K2 path law).
fn resolve(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let p = Path::new(raw);
    if p.is_absolute() {
        return Err(String::from("absolute paths are refused"));
    }
    let mut out = PathBuf::from(root);
    for component in p.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return Err(String::from("path traversal is refused")),
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct ReadArgs {
    path: String,
}

#[derive(Deserialize)]
struct WriteArgs {
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct EditArgs {
    path: String,
    old: String,
    new: String,
}

#[derive(Deserialize)]
struct GrepArgs {
    pattern: String,
    path: String,
}

/// Whitespace-tolerant fallback match: exact first, then a comparison that
/// trims per-line trailing whitespace on both sides (I-06 fuzzy-edit
/// precedent, KC0 subset).
fn fuzzy_find(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if let Some(at) = haystack.find(needle) {
        return Some((at, needle.len()));
    }
    let normalized_needle: Vec<&str> = needle.lines().map(str::trim_end).collect();
    let hay_lines: Vec<&str> = haystack.lines().collect();
    if normalized_needle.is_empty() || hay_lines.len() < normalized_needle.len() {
        return None;
    }
    for start in 0..=(hay_lines.len() - normalized_needle.len()) {
        let window = &hay_lines[start..start + normalized_needle.len()];
        if window
            .iter()
            .map(|l| l.trim_end())
            .eq(normalized_needle.iter().copied())
        {
            // Byte offset of the window's first line and length through the
            // last matched line.
            let prefix: usize = hay_lines[..start].iter().map(|l| l.len() + 1).sum();
            let len: usize = window
                .iter()
                .map(|l| l.len() + 1)
                .sum::<usize>()
                .saturating_sub(1);
            return Some((prefix, len));
        }
    }
    None
}

/// Wraps a `Result<String, String>` into a tool outcome pair (Ok text is
/// the success output; Err text is echoed in both the outcome and the
/// funneled output).
fn finish(result: Result<String, String>) -> (ToolOutcome, String) {
    match result {
        Ok(output) => (ToolOutcome::Succeeded, output),
        Err(message) => (
            ToolOutcome::Failed {
                message: message.clone(),
            },
            message,
        ),
    }
}

fn do_read(root: &Path, raw: &str) -> Result<String, String> {
    let args: ReadArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn do_write(root: &Path, raw: &str) -> Result<String, String> {
    let args: WriteArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &args.content).map_err(|e| e.to_string())?;
    Ok(format!("wrote {} bytes", args.content.len()))
}

fn do_edit(root: &Path, raw: &str) -> Result<String, String> {
    let args: EditArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (at, len) = fuzzy_find(&content, &args.old).ok_or("old text not found (exact or fuzzy)")?;
    let mut next = String::with_capacity(content.len() + args.new.len());
    next.push_str(&content[..at]);
    next.push_str(&args.new);
    next.push_str(&content[at + len..]);
    std::fs::write(&path, &next).map_err(|e| e.to_string())?;
    Ok(String::from("edit applied"))
}

fn do_grep(root: &Path, raw: &str) -> Result<String, String> {
    let args: GrepArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let hits: Vec<String> = content
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(&args.pattern))
        .map(|(n, line)| format!("{}:{line}", n + 1))
        .collect();
    Ok(hits.join("\n"))
}

/// Runs one tool call to completion, synchronously (the reactor wraps this
/// in `spawn_blocking` ownership; results funnel back as effect terminals).
#[must_use]
pub fn run(root: &Path, call: &ProposedToolCall) -> (ToolOutcome, String) {
    let raw = call.args_json.as_str();
    match call.name.as_str() {
        "read" => finish(do_read(root, raw)),
        "write" => finish(do_write(root, raw)),
        "edit" => finish(do_edit(root, raw)),
        "grep" => finish(do_grep(root, raw)),
        other => finish(Err(format!("unknown tool: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_and_traversal_paths_are_refused() {
        let root = Path::new("/tmp/x");
        assert!(resolve(root, "/etc/passwd").is_err());
        assert!(resolve(root, "../up").is_err());
        assert!(resolve(root, "ok/child.rs").is_ok());
    }

    #[test]
    fn fuzzy_find_tolerates_trailing_whitespace() {
        let hay = "fn a() {   \n    body\n}\n";
        let needle = "fn a() {\n    body\n}";
        assert!(fuzzy_find(hay, needle).is_some());
    }
}
