use std::path::{Path, PathBuf};

use crate::error::AppResult;
use tauri::{AppHandle, Manager};

fn workspace_root() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

fn candidate_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(root) = workspace_root() {
        dirs.push(root.join("dist"));
        if let Some(parent) = root.parent() {
            dirs.push(parent.join("dist"));
        }
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        dirs.push(resource_dir.clone());
        dirs.push(resource_dir.join("dist"));
    }
    dirs
}

fn safe_join(base: &Path, relative: &str) -> Option<PathBuf> {
    let trimmed = relative.trim_start_matches('/');
    let relative_path = Path::new(trimmed);
    if relative_path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return None;
    }
    let target = if trimmed.is_empty() {
        base.join("index.html")
    } else {
        base.join(trimmed)
    };
    let canonical_base = base.canonicalize().ok()?;
    let canonical_target = if target.exists() {
        target.canonicalize().ok()?
    } else {
        target
    };
    if canonical_target.starts_with(&canonical_base) {
        Some(canonical_target)
    } else {
        None
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

pub fn load(app: &AppHandle, relative: &str) -> AppResult<Option<(String, Vec<u8>)>> {
    for base in candidate_dirs(app) {
        if !base.exists() {
            continue;
        }

        let requested = safe_join(&base, relative).filter(|path| path.exists()).or_else(|| {
            let trimmed = relative.trim_start_matches('/');
            let request_path = Path::new(trimmed);
            if request_path.extension().is_some() {
                None
            } else {
                safe_join(&base, "index.html").filter(|path| path.exists())
            }
        });

        let Some(path) = requested else {
            continue;
        };

        let body = std::fs::read(&path)?;
        return Ok(Some((content_type(&path).to_string(), body)));
    }

    Ok(None)
}
