use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};

/// 原子写入：在目标同目录写临时文件 → flush → rename 替换，避免半写状态。
///
/// - 自动创建父目录。
/// - Windows 上 `rename` 不允许覆盖已存在文件，故先删目标再 rename。
/// - 失败时清理临时文件。
pub fn atomic_write(path: &Path, data: &[u8]) -> AppResult<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("路径没有父目录: {}", path.display())))?;
    fs::create_dir_all(parent)?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tmpfile");
    let tmp = parent.join(format!(".{file_name}.tmp.{nanos}"));

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.flush()?;
    }

    #[cfg(windows)]
    {
        if path.exists() {
            if let Err(e) = fs::remove_file(path) {
                let _ = fs::remove_file(&tmp);
                return Err(e.into());
            }
        }
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

/// 原子写入字符串。
pub fn atomic_write_str(path: &Path, content: &str) -> AppResult<()> {
    atomic_write(path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ccmesh_atomic_test_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_new_file_and_creates_parent() {
        let dir = tmp_dir();
        let target = dir.join("nested").join("a.txt");
        atomic_write_str(&target, "hello").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrites_existing_file() {
        let dir = tmp_dir();
        let target = dir.join("b.txt");
        atomic_write_str(&target, "first").unwrap();
        atomic_write_str(&target, "second").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "second");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn leaves_no_tmp_residue() {
        let dir = tmp_dir();
        let target = dir.join("c.txt");
        atomic_write_str(&target, "data").unwrap();
        let residue: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.contains(".tmp."))
                    .unwrap_or(false)
            })
            .collect();
        assert!(residue.is_empty(), "临时文件未清理: {residue:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}
