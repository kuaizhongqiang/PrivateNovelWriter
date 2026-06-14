use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
}

/// 确保目录存在
pub fn ensure_dir(path: &Path) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// 读取 .txt 文件内容
pub fn read_text(path: &Path) -> Result<String, StorageError> {
    if !path.exists() {
        return Err(StorageError::NotFound(path.display().to_string()));
    }
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// 写入 .txt 文件（覆盖，原子写入）
///
/// 先写入同目录下的 .tmp 临时文件，然后 rename 到目标路径。
/// 同文件系统下 rename 是原子操作，避免写入中途崩溃产生脏数据。
pub fn write_text(path: &Path, content: &str) -> Result<(), StorageError> {
    ensure_dir(path)?;
    let tmp_path = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?; // 确保数据刷到磁盘
    }
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// 启动时清理残留的 .tmp 文件（由原子写入中断产生）
///
/// 扫描 text/ 目录下所有 .tmp 文件并将其删除。
pub fn cleanup_orphan_tmp(project_root: &Path) -> Result<usize, StorageError> {
    let text_dir = project_root.join("text");
    if !text_dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    walkdir(&text_dir, &mut count)?;
    Ok(count)
}

fn walkdir(dir: &Path, count: &mut usize) -> Result<(), StorageError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walkdir(&path, count)?;
        } else if path.extension().map_or(false, |e| e == "tmp") {
            fs::remove_file(&path)?;
            *count += 1;
        }
    }
    Ok(())
}

/// 统计中文字数（含标点，不含空白）
pub fn count_chars(text: &str) -> i32 {
    text.chars().filter(|c| !c.is_whitespace()).count() as i32
}

/// 删除文件
pub fn delete_file(path: &Path) -> Result<(), StorageError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_atomic_write() {
        let dir = std::env::temp_dir().join("pnw_test_atomic");
        let path = dir.join("ch-001.txt");
        let content = "原子写入测试内容。";

        write_text(&path, content).unwrap();
        assert!(path.exists());
        // 验证内容正确写入
        assert_eq!(read_text(&path).unwrap(), content);

        // 模拟 text/ 目录下的中断残留并清理
        let text_dir = dir.join("text");
        fs::create_dir_all(&text_dir).unwrap();
        let tmp_path = text_dir.join("orphan.tmp");
        fs::write(&tmp_path, "residue").unwrap();
        let cleaned = cleanup_orphan_tmp(&dir).unwrap();
        assert_eq!(cleaned, 1);
        assert!(!tmp_path.exists());

        // 清理后再次清理应返回 0
        let cleaned2 = cleanup_orphan_tmp(&dir).unwrap();
        assert_eq!(cleaned2, 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_and_read_text() {
        let dir = std::env::temp_dir().join("pnw_test_storage");
        let path = dir.join("test.txt");
        let content = "这是一段测试正文内容。";

        write_text(&path, content).unwrap();
        let read = read_text(&path).unwrap();
        assert_eq!(read, content);

        delete_file(&path).unwrap();
        assert!(!path.exists());

        fs::remove_dir(&dir).ok();
    }

    #[test]
    fn test_count_chars() {
        assert_eq!(count_chars("你好世界"), 4);
        assert_eq!(count_chars("Hello World"), 10);
        assert_eq!(count_chars(" "), 0);
    }

    #[test]
    fn test_read_nonexistent() {
        let path = PathBuf::from("/nonexistent/path.txt");
        assert!(read_text(&path).is_err());
    }
}
