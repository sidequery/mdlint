use crate::config::{Config, WikilinkResolution};
use crate::headings::slug_matches;
use crate::links::{Link, LinkKind};
use crate::workspace::Workspace;
use percent_encoding::percent_decode_str;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ResolveError {
    FileNotFound {
        target: String,
    },
    AnchorNotFound {
        file: PathBuf,
        anchor: String,
    },
    AmbiguousWikilink {
        target: String,
        candidates: Vec<PathBuf>,
    },
    CaseMismatch {
        target: String,
        actual: PathBuf,
    },
}

/// Resolve a link within the workspace. Returns Ok(resolved path) or Err(reason).
pub fn resolve_link(
    link: &Link,
    source_file: &Path,
    workspace: &Workspace,
    config: &Config,
) -> Result<Option<PathBuf>, ResolveError> {
    // Skip external URLs
    if link.is_external() {
        return Ok(None);
    }

    match link.kind {
        LinkKind::WikiLink => resolve_wikilink(link, source_file, workspace, config),
        LinkKind::Standard | LinkKind::Image => {
            resolve_standard_link(link, source_file, workspace, config)
        }
    }
}

fn resolve_wikilink(
    link: &Link,
    source_file: &Path,
    workspace: &Workspace,
    config: &Config,
) -> Result<Option<PathBuf>, ResolveError> {
    let target = link.file_target.as_deref().unwrap_or("");
    if target.is_empty() {
        // Anchor-only wikilink within same file
        if let Some(ref anchor) = link.anchor {
            check_anchor(source_file, anchor, workspace, config)?;
        }
        return Ok(Some(source_file.to_path_buf()));
    }

    let resolved = match config.links.wikilink_resolution {
        WikilinkResolution::ShortestPath => {
            resolve_wikilink_shortest(target, source_file, workspace, config)?
        }
        WikilinkResolution::Relative => {
            resolve_relative(target, source_file, workspace)?
        }
    };

    // Check anchor if present
    if let Some(ref anchor) = link.anchor {
        check_anchor(&resolved, anchor, workspace, config)?;
    }

    Ok(Some(resolved))
}

fn resolve_wikilink_shortest(
    target: &str,
    _source_file: &Path,
    workspace: &Workspace,
    config: &Config,
) -> Result<PathBuf, ResolveError> {
    let target_lower = target.to_lowercase();

    // Check basename index
    if let Some(indices) = workspace.basename_index.get(&target_lower) {
        if indices.len() == 1 {
            let file = &workspace.files[indices[0]];
            // Check for case mismatch
            if config.links.warn_case_mismatch {
                let actual_stem = file
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if actual_stem != target && actual_stem.to_lowercase() == target_lower {
                    return Err(ResolveError::CaseMismatch {
                        target: target.to_string(),
                        actual: file.path.clone(),
                    });
                }
            }
            return Ok(file.path.clone());
        } else if indices.len() > 1 {
            let candidates: Vec<PathBuf> =
                indices.iter().map(|&i| workspace.files[i].path.clone()).collect();
            return Err(ResolveError::AmbiguousWikilink {
                target: target.to_string(),
                candidates,
            });
        }
    }

    // Check alias index
    if let Some(indices) = workspace.alias_index.get(&target_lower) {
        if indices.len() == 1 {
            return Ok(workspace.files[indices[0]].path.clone());
        } else if indices.len() > 1 {
            let candidates: Vec<PathBuf> =
                indices.iter().map(|&i| workspace.files[i].path.clone()).collect();
            return Err(ResolveError::AmbiguousWikilink {
                target: target.to_string(),
                candidates,
            });
        }
    }

    // Try with .md extension as a path from workspace root
    let target_with_ext = if !target_lower.ends_with(".md") {
        format!("{}.md", target)
    } else {
        target.to_string()
    };

    let candidate = workspace.root.join(&target_with_ext);
    if workspace.known_paths.contains(&candidate) {
        return Ok(candidate);
    }

    Err(ResolveError::FileNotFound {
        target: target.to_string(),
    })
}

fn resolve_standard_link(
    link: &Link,
    source_file: &Path,
    workspace: &Workspace,
    config: &Config,
) -> Result<Option<PathBuf>, ResolveError> {
    let target = match &link.file_target {
        Some(t) => t,
        None => {
            // Anchor-only link
            if let Some(ref anchor) = link.anchor {
                check_anchor(source_file, anchor, workspace, config)?;
            }
            return Ok(Some(source_file.to_path_buf()));
        }
    };

    let resolved = resolve_relative(target, source_file, workspace)?;

    // Check anchor if present
    if let Some(ref anchor) = link.anchor {
        check_anchor(&resolved, anchor, workspace, config)?;
    }

    Ok(Some(resolved))
}

fn resolve_relative(
    target: &str,
    source_file: &Path,
    workspace: &Workspace,
) -> Result<PathBuf, ResolveError> {
    let decoded = percent_decode_str(target).decode_utf8_lossy();
    let target_path = Path::new(decoded.as_ref());

    let base_dir = source_file.parent().unwrap_or(Path::new("."));
    let resolved = base_dir.join(target_path);

    // O(1) lookup: exact path in known_paths
    if workspace.known_paths.contains(&resolved) {
        return Ok(resolved);
    }

    // O(1) lookup: try with .md extension
    if !target.ends_with(".md") {
        let with_ext = resolved.with_extension("md");
        if workspace.known_paths.contains(&with_ext) {
            return Ok(with_ext);
        }
    }

    // O(1) lookup: case-insensitive via path_case_index
    let lower_key = resolved.to_string_lossy().to_lowercase();
    if let Some(actual) = workspace.path_case_index.get(&lower_key) {
        return Ok(actual.clone());
    }

    // Case-insensitive with .md extension
    if !target.ends_with(".md") {
        let with_ext = resolved.with_extension("md");
        let lower_ext_key = with_ext.to_string_lossy().to_lowercase();
        if let Some(actual) = workspace.path_case_index.get(&lower_ext_key) {
            return Ok(actual.clone());
        }
    }

    Err(ResolveError::FileNotFound {
        target: target.to_string(),
    })
}

fn check_anchor(
    file_path: &Path,
    anchor: &str,
    workspace: &Workspace,
    config: &Config,
) -> Result<(), ResolveError> {
    let headings = match workspace.heading_index.get(file_path) {
        Some(h) => h,
        None => {
            // File not in workspace (maybe a non-md file), skip anchor check
            return Ok(());
        }
    };

    let mode = config.links.slug_mode;
    let found = headings.iter().any(|h| slug_matches(h, anchor, mode));
    if found {
        Ok(())
    } else {
        Err(ResolveError::AnchorNotFound {
            file: file_path.to_path_buf(),
            anchor: anchor.to_string(),
        })
    }
}
