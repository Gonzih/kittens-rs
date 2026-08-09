//! Tool discharge over the real filesystem (SPEC K1/K2, KC0 subset).
//!
//! Paths are workspace-relative and normalized: absolute paths, `..`
//! traversal, and symlink escapes are refused (K2). Writes and edits use a
//! synced create-new temp in the canonical target parent followed by a
//! same-directory rename, so the target is always the old or complete new
//! file. Existing components and the replacement target leaf are rechecked
//! for symlinks, but KC0 does not claim immunity to a hostile concurrent
//! ancestor swap or an edit-source leaf swap between metadata check and
//! read-open: without handle-relative `openat`/`O_NOFOLLOW` operations a
//! residual TOCTOU remains.
//! `exec` is deliberately absent from this KC0 build (feature-gated
//! capability; K3 rule — a tool not present is a tool not advertised).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use kittens_code_core::engine::ProposedToolCall;
use kittens_code_protocol::event::ToolOutcome;
use serde::Deserialize;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

struct TempPath {
    path: Option<PathBuf>,
}

impl TempPath {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Resolves a workspace-relative path or refuses it (K2 path law).
///
/// Lexical checks reject absolute paths and `..`/root traversal, but lexical
/// normalization alone is not containment: a symlink component can still
/// escape the root (review input 19 #7). So every existing component of the
/// resolved path — and the final path itself when it exists — is refused if
/// it is a symlink, and (defense in depth) the canonicalized ancestor that
/// exists must stay within the canonicalized root.
fn resolve(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let p = Path::new(raw);
    if p.is_absolute() {
        return Err(String::from("absolute paths are refused"));
    }
    let mut out = PathBuf::from(root);
    for component in p.components() {
        match component {
            Component::Normal(part) => {
                out.push(part);
                // Refuse any existing symlink component: following it could
                // leave the root. A not-yet-existing component is fine (it
                // will be created inside the root by write/edit).
                if let Ok(meta) = std::fs::symlink_metadata(&out) {
                    if meta.file_type().is_symlink() {
                        return Err(String::from("symlinked path components are refused"));
                    }
                }
            }
            Component::CurDir => {}
            _ => return Err(String::from("path traversal is refused")),
        }
    }
    // Containment: the deepest existing ancestor, canonicalized, must be
    // inside the canonicalized root.
    let real_root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut probe = out.as_path();
    while !probe.exists() {
        match probe.parent() {
            Some(parent) => probe = parent,
            None => break,
        }
    }
    if let Ok(real) = std::fs::canonicalize(probe) {
        if !real.starts_with(&real_root) {
            return Err(String::from("resolved path escapes the workspace root"));
        }
    }
    Ok(out)
}

/// Returns a canonical in-root parent and its target leaf. Canonicalizing
/// immediately before temp creation reduces the component-swap window and
/// ensures the temp and rename target are on the same filesystem.
fn canonical_target(root: &Path, target: &Path) -> Result<(PathBuf, PathBuf), String> {
    let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let parent = target
        .parent()
        .ok_or_else(|| String::from("target has no parent"))?;
    let parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !parent.starts_with(&root) {
        return Err(String::from("target parent escapes the workspace root"));
    }
    let leaf = target
        .file_name()
        .ok_or_else(|| String::from("target must name a file"))?;
    Ok((parent.clone(), parent.join(leaf)))
}

fn refuse_symlink_leaf(target: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(String::from("symlinked write target is refused"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn create_temp(parent: &Path) -> Result<(File, TempPath), String> {
    create_temp_with_limit(parent, 128, || NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed))
}

fn temp_path(parent: &Path, id: u64) -> PathBuf {
    parent.join(format!(".kittens-code-{}-{id}.tmp", std::process::id()))
}

fn create_temp_with_limit<F>(
    parent: &Path,
    attempts: usize,
    mut next_id: F,
) -> Result<(File, TempPath), String>
where
    F: FnMut() -> u64,
{
    for _ in 0..attempts {
        let path = temp_path(parent, next_id());
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, TempPath::new(path))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(String::from("could not allocate an atomic-write temp file"))
}

/// Atomic replacement primitive with a hook after the complete temp has been
/// synced but before the final leaf check/rename. Production passes a no-op;
/// tests use the hook to model interruption and a target-leaf swap at the
/// exact adversarial boundary.
fn atomic_replace_with<F>(
    root: &Path,
    target: &Path,
    content: &[u8],
    before_rename: F,
) -> Result<(), String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let (parent, target) = canonical_target(root, target)?;
    let (mut temp, mut guard) = create_temp(&parent)?;
    temp.write_all(content).map_err(|e| e.to_string())?;
    temp.sync_all().map_err(|e| e.to_string())?;
    drop(temp);

    before_rename(&target)?;
    refuse_symlink_leaf(&target)?;
    let temp_path = guard
        .path
        .as_ref()
        .ok_or_else(|| String::from("atomic-write temp was already consumed"))?;
    std::fs::rename(temp_path, &target).map_err(|e| e.to_string())?;
    guard.disarm();
    Ok(())
}

fn atomic_replace(root: &Path, target: &Path, content: &[u8]) -> Result<(), String> {
    atomic_replace_with(root, target, content, |_| Ok(()))
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

fn read_checked(root: &Path, target: &Path) -> Result<String, String> {
    let (_, target) = canonical_target(root, target)?;
    refuse_symlink_leaf(&target)?;
    std::fs::read_to_string(target).map_err(|e| e.to_string())
}

fn do_write(root: &Path, raw: &str) -> Result<String, String> {
    let args: WriteArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Repeat all component checks after directory creation: a symlink
    // inserted during create_dir_all is refused before the temp is opened.
    let path = resolve(root, &args.path)?;
    atomic_replace(root, &path, args.content.as_bytes())?;
    Ok(format!("wrote {} bytes", args.content.len()))
}

fn do_edit(root: &Path, raw: &str) -> Result<String, String> {
    let args: EditArgs = serde_json::from_str(raw).map_err(|e| format!("bad arguments: {e}"))?;
    let path = resolve(root, &args.path)?;
    let content = read_checked(root, &path)?;
    let (at, len) = fuzzy_find(&content, &args.old).ok_or("old text not found (exact or fuzzy)")?;
    let mut next = String::with_capacity(content.len() + args.new.len());
    next.push_str(&content[..at]);
    next.push_str(&args.new);
    next.push_str(&content[at + len..]);
    atomic_replace(root, &path, next.as_bytes())?;
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
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        assert!(resolve(root, "/etc/passwd").is_err());
        assert!(resolve(root, "../up").is_err());
        // A not-yet-existing child inside the root resolves.
        assert!(resolve(root, "ok/child.rs").is_ok());
    }

    #[test]
    fn symlink_components_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("secret"), "top secret").expect("seed secret");
        // A symlink inside the root pointing outside it.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path(), root.join("escape")).expect("symlink");
            // Reaching through the symlink must be refused (K2 / review #7).
            assert!(resolve(root, "escape/secret").is_err());
            // The symlink itself as a leaf is also refused.
            assert!(resolve(root, "escape").is_err());
        }
    }

    #[test]
    fn fuzzy_find_tolerates_trailing_whitespace() {
        let hay = "fn a() {   \n    body\n}\n";
        let needle = "fn a() {\n    body\n}";
        assert!(fuzzy_find(hay, needle).is_some());
    }

    #[test]
    fn empty_and_nested_traversal_paths_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        // Empty path resolves to the root itself — allowed to resolve, but a
        // read/write on a directory fails downstream; the interesting cases
        // are the escapes.
        assert!(resolve(root, "a/../../etc/passwd").is_err());
        assert!(resolve(root, "ok/../../..").is_err());
        // A `.`-laden but in-bounds path is fine.
        assert!(resolve(root, "./a/./b/c.rs").is_ok());
    }

    #[test]
    fn symlink_leaf_at_write_target_is_refused() {
        // The final component being a symlink (a swap attack on the write
        // target) is refused, not followed (review input 19 #25).
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let outside = tempfile::tempdir().expect("outside");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path().join("target"), root.join("leaf"))
                .expect("symlink");
            assert!(resolve(root, "leaf").is_err());
        }
    }

    #[test]
    fn interrupted_atomic_write_leaves_old_then_complete_new_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let target = root.join("state.txt");
        std::fs::write(&target, "old complete content").expect("seed target");

        let interrupted = atomic_replace_with(root, &target, b"new complete content", |_| {
            Err(String::from("simulated interruption before rename"))
        });
        assert!(interrupted.is_err());
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "old complete content"
        );

        atomic_replace(root, &target, b"new complete content").expect("atomic replacement");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "new complete content"
        );
    }

    #[test]
    fn interrupted_edit_replacement_leaves_old_content_and_edit_uses_atomic_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let target = root.join("edit.txt");
        std::fs::write(&target, "alpha old omega").expect("seed target");

        let interrupted = atomic_replace_with(root, &target, b"alpha new omega", |_| {
            Err(String::from("simulated interruption before edit rename"))
        });
        assert!(interrupted.is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "alpha old omega");

        let call = ProposedToolCall {
            name: String::from("edit"),
            args_json: serde_json::json!({
                "path": "edit.txt",
                "old": "old",
                "new": "new"
            })
            .to_string(),
        };
        assert_eq!(run(root, &call).0, ToolOutcome::Succeeded);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "alpha new omega");
    }

    #[test]
    fn canonical_target_refuses_an_outside_parent_and_non_directory_temp_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        assert!(canonical_target(dir.path(), &outside.path().join("file")).is_err());
        assert!(canonical_target(dir.path(), Path::new("/")).is_err());

        let not_a_directory = dir.path().join("plain-file");
        std::fs::write(&not_a_directory, "x").expect("seed file");
        assert!(create_temp(&not_a_directory).is_err());
        assert!(create_temp_with_limit(dir.path(), 0, || 0).is_err());
        assert!(refuse_symlink_leaf(Path::new("bad\0path")).is_err());

        let collision = temp_path(dir.path(), u64::MAX);
        std::fs::write(&collision, "occupied").expect("seed colliding temp name");
        let mut ids = [u64::MAX, u64::MAX - 1].into_iter();
        let (_, guard) = create_temp_with_limit(dir.path(), 2, || ids.next().unwrap())
            .expect("collision retries with the next temp id");
        assert_eq!(
            guard.path.as_deref(),
            Some(temp_path(dir.path(), u64::MAX - 1).as_path())
        );
    }

    #[test]
    fn target_swapped_to_symlink_before_rename_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let target = root.join("victim.txt");
        let outside = tempfile::tempdir().expect("outside");
        let outside_target = outside.path().join("outside.txt");
        std::fs::write(&target, "old inside").expect("seed inside");
        std::fs::write(&outside_target, "outside must survive").expect("seed outside");

        #[cfg(unix)]
        {
            let result = atomic_replace_with(root, &target, b"new inside", |checked_target| {
                std::fs::remove_file(checked_target).map_err(|e| e.to_string())?;
                std::os::unix::fs::symlink(&outside_target, checked_target)
                    .map_err(|e| e.to_string())
            });
            assert!(result.is_err());
            assert_eq!(
                std::fs::read_to_string(&outside_target).unwrap(),
                "outside must survive"
            );
            assert!(
                std::fs::symlink_metadata(&target)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }

    #[test]
    fn write_creates_nested_target_through_atomic_replacement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let call = ProposedToolCall {
            name: String::from("write"),
            args_json: serde_json::json!({
                "path": "nested/file.txt",
                "content": "complete"
            })
            .to_string(),
        };
        let (outcome, output) = run(dir.path(), &call);
        assert_eq!(outcome, ToolOutcome::Succeeded);
        assert_eq!(output, "wrote 8 bytes");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("nested/file.txt")).unwrap(),
            "complete"
        );
    }

    #[test]
    fn malformed_tool_json_is_a_tool_failure_not_a_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let call = ProposedToolCall {
            name: String::from("read"),
            args_json: String::from("{not valid json"),
        };
        let (outcome, _) = run(dir.path(), &call);
        assert!(matches!(outcome, ToolOutcome::Failed { .. }));
    }

    #[test]
    fn fuzzy_find_tolerates_crlf_line_endings() {
        let hay = "fn a() {\r\n    body\r\n}\r\n";
        let needle = "fn a() {\n    body\n}";
        assert!(
            fuzzy_find(hay, needle).is_some(),
            "trailing-whitespace normalization also absorbs CR"
        );
    }
}
