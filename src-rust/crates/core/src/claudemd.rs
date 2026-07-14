//! AGENTS.md hierarchical memory loading.
//! Mirrors src/utils/claudemd.ts (1,479 lines).
//!
//! Priority order: managed > user > project > local
//! Supports @include directives, YAML frontmatter, and mtime-based caching.
//!
//! Framework files are loaded with delivery modes:
//!   SessionStart — injected once at session creation (cacheable)
//!   EveryTurn    — injected every turn in the dynamic block

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Memory file type / priority scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    /// `~/.claurst/rules/*.md` — global managed policy.
    Managed,
    /// `~/.claurst/AGENTS.md` — user-level memory.
    User,
    /// `{project_root}/AGENTS.md` — project-level memory.
    Project,
    /// `{project_root}/.claurst/AGENTS.md` — local override.
    Local,
}

/// When a framework file is injected into the system prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// Sent once at session start (cacheable block).
    SessionStart,
    /// Sent every turn in the dynamic block.
    EveryTurn,
}

/// Frontmatter parsed from a AGENTS.md file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryFrontmatter {
    #[serde(default)]
    pub memory_type: Option<String>,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Loaded memory file with metadata.
#[derive(Debug, Clone)]
pub struct MemoryFileInfo {
    pub path: PathBuf,
    pub scope: MemoryScope,
    pub content: String,
    pub frontmatter: MemoryFrontmatter,
    pub mtime: Option<SystemTime>,
    /// When this file is delivered to the system prompt.
    pub delivery: DeliveryMode,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Simple mtime-keyed file cache.
#[derive(Default)]
pub struct MemoryCache {
    entries: HashMap<PathBuf, (SystemTime, String)>,
}

impl MemoryCache {
    /// Return cached content if the file hasn't changed since last read.
    pub fn get(&self, path: &Path) -> Option<&str> {
        let mtime = std::fs::metadata(path).ok()?.modified().ok()?;
        let (cached_mtime, content) = self.entries.get(path)?;
        if *cached_mtime == mtime { Some(content.as_str()) } else { None }
    }

    /// Store file content with its current mtime.
    pub fn insert(&mut self, path: PathBuf, content: String) {
        if let Ok(mtime) = std::fs::metadata(&path).and_then(|m| m.modified()) {
            self.entries.insert(path, (mtime, content));
        }
    }
}

// ---------------------------------------------------------------------------
// YAML frontmatter parsing
// ---------------------------------------------------------------------------

/// Strip YAML frontmatter (--- ... ---) from content and parse it.
/// Returns (frontmatter, body_without_frontmatter).
pub fn parse_frontmatter(content: &str) -> (MemoryFrontmatter, &str) {
    if !content.starts_with("---") {
        return (MemoryFrontmatter::default(), content);
    }
    let after_first = &content[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml = after_first[..end].trim();
        let body = &after_first[end + 4..];
        // Minimal YAML key-value parse (no external dependency).
        let mut fm = MemoryFrontmatter::default();
        for line in yaml.lines() {
            let line = line.trim();
            if let Some((key, val)) = line.split_once(':') {
                let val = val.trim().to_string();
                match key.trim() {
                    "memory_type" => fm.memory_type = Some(val),
                    "priority" => fm.priority = val.parse().ok(),
                    "scope" => fm.scope = Some(val),
                    _ => {}
                }
            }
        }
        return (fm, body.trim_start_matches('\n'));
    }
    (MemoryFrontmatter::default(), content)
}

// ---------------------------------------------------------------------------
// @include directive expansion
// ---------------------------------------------------------------------------

/// Maximum @include nesting depth.
const MAX_INCLUDE_DEPTH: usize = 10;

/// Expand @include directives in content.
/// Circular references are detected via `visited` set.
pub fn expand_includes(
    content: &str,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> String {
    if depth >= MAX_INCLUDE_DEPTH {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(path_str) = trimmed.strip_prefix("@include ") {
            let path_str = path_str.trim();
            // Resolve relative to base_dir; expand ~ to home dir.
            let include_path = if path_str.starts_with('~') {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(&path_str[2..])
            } else if Path::new(path_str).is_absolute() {
                PathBuf::from(path_str)
            } else {
                base_dir.join(path_str)
            };

            let canonical = include_path.canonicalize().unwrap_or(include_path.clone());
            if visited.contains(&canonical) {
                result.push_str(&format!("<!-- circular @include {} skipped -->\n", path_str));
                continue;
            }
            if let Ok(included) = std::fs::read_to_string(&include_path) {
                // Check max size.
                if included.len() > 40 * 1024 {
                    result.push_str(&format!("<!-- @include {} exceeds 40KB limit -->\n", path_str));
                    continue;
                }
                visited.insert(canonical);
                let expanded = expand_includes(
                    &included,
                    include_path.parent().unwrap_or(base_dir),
                    visited,
                    depth + 1,
                );
                result.push_str(&expanded);
                result.push('\n');
            } else {
                result.push_str(&format!("<!-- @include {} not found -->\n", path_str));
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Loading API
// ---------------------------------------------------------------------------

const MAX_FILE_SIZE: u64 = 40 * 1024; // 40 KB

/// Load a single framework file (respects MAX_FILE_SIZE, expands @includes).
pub fn load_memory_file(
    path: &Path,
    scope: MemoryScope,
    delivery: DeliveryMode,
) -> Option<MemoryFileInfo> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_FILE_SIZE {
        eprintln!("WARNING: {} exceeds 40KB limit, skipping", path.display());
        return None;
    }
    let raw = std::fs::read_to_string(path).ok()?;
    let mtime = meta.modified().ok();

    let (frontmatter, body) = parse_frontmatter(&raw);
    let mut visited = HashSet::new();
    visited.insert(path.canonicalize().unwrap_or(path.to_path_buf()));
    let content = expand_includes(body, path.parent().unwrap_or(Path::new(".")), &mut visited, 0);

    Some(MemoryFileInfo {
        path: path.to_path_buf(),
        scope,
        content,
        frontmatter,
        mtime,
        delivery,
    })
}

/// Framework file map entry.
struct FrameworkFile {
    name: &'static str,
    delivery: DeliveryMode,
}

/// Load framework files from a directory for a given scope.
///
/// Uses the framework file map instead of a simple name list.
/// Cascaded files (AGENTS.md, AGENT.md, USER.md) are handled by
/// `load_cascaded_file` — they do not appear in the scope tables.
fn load_scope_files(dir: &Path, scope: MemoryScope, files: &mut Vec<MemoryFileInfo>) {
    let file_map: &[FrameworkFile] = match scope {
        // Managed scope: keep original wildcard *.md loading (handled separately).
        MemoryScope::Managed => &[],

        // User scope: empty — USER.md is cascaded (global → project).
        MemoryScope::User => &[],

        // Project scope: empty — AGENTS.md is cascaded (global → project).
        MemoryScope::Project => &[],

        // Local scope ({project_root}/.claurst/): project-only framework files.
        // Cascaded files (AGENT.md, AGENTS.md, USER.md, BRAIN.md) are not in this table.
        MemoryScope::Local => &[
            FrameworkFile { name: "ATTRACTOR.md", delivery: DeliveryMode::SessionStart },
            FrameworkFile { name: "HEART.md", delivery: DeliveryMode::SessionStart },
            FrameworkFile { name: "MEMORY.md", delivery: DeliveryMode::SessionStart },
            FrameworkFile { name: "STATE.md", delivery: DeliveryMode::EveryTurn },
        ],
    };

    for entry in file_map {
        let path = dir.join(entry.name);
        if path.exists() {
            if let Some(f) = load_memory_file(&path, scope, entry.delivery) {
                files.push(f);
            }
        }
    }
}

/// Cascade load: try global (~/.claurst/name) first, fall back to project ({root}/name).
///
/// Used for AGENTS.md, AGENT.md, USER.md, and BRAIN.md — the four framework files
/// that support a global override. Global files get `MemoryScope::User`; project
/// fallback files get `MemoryScope::Project`.
fn load_cascaded_file(
    global_dir: &Path,
    project_dir: &Path,
    name: &str,
    delivery: DeliveryMode,
) -> Option<MemoryFileInfo> {
    let global_path = global_dir.join(name);
    if global_path.exists() {
        return load_memory_file(&global_path, MemoryScope::User, delivery);
    }
    let project_path = project_dir.join(name);
    if project_path.exists() {
        return load_memory_file(&project_path, MemoryScope::Project, delivery);
    }
    None
}

/// Load all framework files for the given project root, in priority order.
///
/// Returns a tuple of (session_start_files, every_turn_files).
/// Session-start files go into the cacheable system prompt block.
/// Every-turn files go into the dynamic memory block.
///
/// Loading order:
///   1. Managed: ~/.claurst/rules/*.md (EveryTurn)
///   2. Cascaded SessionStart: AGENTS.md, AGENT.md, USER.md, BRAIN.md (global → project)
///   3. Project-only SessionStart: ATTRACTOR.md, HEART.md, MEMORY.md
///   4. EveryTurn duplicates: ATTRACTOR.md, HEART.md, MEMORY.md (SessionStart + EveryTurn)
///   5. Project-only EveryTurn: STATE.md
pub fn load_all_memory_files(
    project_root: &Path,
) -> (Vec<MemoryFileInfo>, Vec<MemoryFileInfo>) {
    let mut all = Vec::new();

    // 1. Managed: ~/.claurst/rules/*.md (EveryTurn delivery)
    {
        let claurst = crate::config::Settings::config_dir();
        let rules_dir = claurst.join("rules");
        if let Ok(entries) = std::fs::read_dir(&rules_dir) {
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    if p.extension().is_some_and(|x| x == "md") { Some(p) } else { None }
                })
                .collect();
            paths.sort();
            for p in paths {
                if let Some(f) = load_memory_file(&p, MemoryScope::Managed, DeliveryMode::EveryTurn) {
                    all.push(f);
                }
            }
        }

        let global_dir = claurst.clone();

        // 2. Cascaded SessionStart: AGENTS.md, AGENT.md, USER.md, BRAIN.md (global → project)
        for name in &["AGENTS.md", "AGENT.md", "USER.md", "BRAIN.md"] {
            if let Some(f) = load_cascaded_file(
                &global_dir, project_root, name, DeliveryMode::SessionStart,
            ) {
                all.push(f);
            }
        }

        // 3. Project-only SessionStart: ATTRACTOR.md, HEART.md, MEMORY.md
        load_scope_files(&project_root.join(".claurst"), MemoryScope::Local, &mut all);
        // Also load project-root copies of the project-only session-start files
        for name in &["ATTRACTOR.md", "HEART.md", "MEMORY.md"] {
            let path = project_root.join(name);
            if path.exists() {
                if let Some(f) = load_memory_file(&path, MemoryScope::Project, DeliveryMode::SessionStart) {
                    all.push(f);
                }
            }
        }

        // 4. EveryTurn duplicates for SessionStart + EveryTurn files.
        //    ATTRACTOR.md, HEART.md, MEMORY.md are already loaded above with
        //    SessionStart delivery (injected into <framework_identity>). Loading
        //    them again with EveryTurn delivery ensures their names appear in
        //    the periodic nudge file list for agent re-reading.
        for name in &["ATTRACTOR.md", "HEART.md", "MEMORY.md"] {
            // Check .claurst/ first
            let local_path = project_root.join(".claurst").join(name);
            if local_path.exists() {
                if let Some(f) = load_memory_file(&local_path, MemoryScope::Local, DeliveryMode::EveryTurn) {
                    all.push(f);
                }
            }
            // Then project root
            let root_path = project_root.join(name);
            if root_path.exists() {
                if let Some(f) = load_memory_file(&root_path, MemoryScope::Project, DeliveryMode::EveryTurn) {
                    all.push(f);
                }
            }
        }

        // 5. Project-only EveryTurn: STATE.md
        let state_path = project_root.join("STATE.md");
        if state_path.exists() {
            if let Some(f) = load_memory_file(&state_path, MemoryScope::Project, DeliveryMode::EveryTurn) {
                all.push(f);
            }
        }
    }

    // Split by delivery mode.

    // Split by delivery mode.
    let session_start: Vec<_> = all
        .iter()
        .filter(|f| f.delivery == DeliveryMode::SessionStart && !f.content.trim().is_empty())
        .cloned()
        .collect();
    let every_turn: Vec<_> = all
        .iter()
        .filter(|f| f.delivery == DeliveryMode::EveryTurn && !f.content.trim().is_empty())
        .cloned()
        .collect();

    (session_start, every_turn)
}

/// Concatenate framework files of a specific delivery mode into a prompt block.
pub fn build_framework_identity(session_start_files: &[MemoryFileInfo]) -> String {
    session_start_files
        .iter()
        .filter(|f| !f.content.trim().is_empty())
        .map(|f| f.content.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Concatenate every-turn memory files into a prompt block.
pub fn build_memory_prompt(every_turn_files: &[MemoryFileInfo]) -> String {
    every_turn_files
        .iter()
        .filter(|f| !f.content.trim().is_empty())
        .map(|f| f.content.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Build the periodic nudge string listing files to re-read.
pub fn build_periodic_nudge(file_names: &[String]) -> Option<String> {
    if file_names.is_empty() {
        return None;
    }
    let list = file_names.join(", ");
    Some(format!(
        "\n<periodic_nudge>\nBe proactive about your memory files: {{MEMORY.md, BRAIN.md}} Re-read the following files to refresh your context: {}\n</periodic_nudge>",
        list
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_basic() {
        let content = "---\nmemory_type: project\npriority: 10\n---\nHello world";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.memory_type.as_deref(), Some("project"));
        assert_eq!(fm.priority, Some(10));
        assert_eq!(body.trim(), "Hello world");
    }

    #[test]
    fn parse_frontmatter_none() {
        let content = "No frontmatter here";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.memory_type.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn load_agents_md_session_start() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "agents content").unwrap();

        let (session_start, _every_turn) = load_all_memory_files(tmp.path());
        // AGENTS.md at project root should be loaded as SessionStart.
        let project: Vec<_> = session_start.iter().filter(|f| f.path.starts_with(tmp.path())).collect();
        assert_eq!(project.len(), 1, "AGENTS.md should be loaded as session-start");
        assert!(project[0].path.ends_with("AGENTS.md"));
        assert_eq!(project[0].delivery, DeliveryMode::SessionStart);
    }

    #[test]
    fn load_local_framework_files() {
        let tmp = tempfile::tempdir().unwrap();
        let local = tmp.path().join(".claurst");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(local.join("AGENT.md"), "agent content").unwrap();
        std::fs::write(local.join("HEART.md"), "heart content").unwrap();
        std::fs::write(local.join("MEMORY.md"), "memory content").unwrap();

        let (session_start, every_turn) = load_all_memory_files(tmp.path());
        // AGENT.md is now cascaded (global → project), not loaded from .claurst/
        // HEART.md: SessionStart (from local) + EveryTurn (duplicates)
        // MEMORY.md: SessionStart (from local) + EveryTurn (duplicates)
        let local_ss: Vec<_> = session_start.iter().filter(|f| f.path.starts_with(&local)).collect();
        let local_et: Vec<_> = every_turn.iter().filter(|f| f.path.starts_with(&local)).collect();
        
        // SessionStart: HEART.md + MEMORY.md = 2
        assert_eq!(local_ss.len(), 2, "Expected HEART.md + MEMORY.md in session-start");
        // EveryTurn: HEART.md + MEMORY.md = 2
        assert_eq!(local_et.len(), 2, "Expected HEART.md + MEMORY.md in every-turn");
    }

    #[test]
    fn claude_md_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "claude only").unwrap();
        std::fs::write(tmp.path().join("AGENTS.md"), "agents content").unwrap();

        let (session_start, every_turn) = load_all_memory_files(tmp.path());
        let project: Vec<_> = session_start.iter()
            .filter(|f| f.path.starts_with(tmp.path()))
            .collect();
        let et_project: Vec<_> = every_turn.iter()
            .filter(|f| f.path.starts_with(tmp.path()))
            .collect();
        // CLAUDE.md is no longer loaded. Only AGENTS.md should appear.
        assert_eq!(project.len(), 1, "Only AGENTS.md should load, not CLAUDE.md");
        assert!(project[0].path.ends_with("AGENTS.md"));
        assert!(et_project.is_empty(), "No every-turn files at project root");
    }

    #[test]
    fn expand_includes_circular() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.md");
        let b = tmp.path().join("b.md");
        std::fs::write(&a, "@include b.md\n").unwrap();
        std::fs::write(&b, "@include a.md\ncontent\n").unwrap();
        let result = expand_includes("@include a.md\n", tmp.path(), &mut std::collections::HashSet::new(), 0);
        // Should not infinite-loop; circular reference comment present.
        assert!(result.contains("circular") || result.contains("content"));
    }
}
