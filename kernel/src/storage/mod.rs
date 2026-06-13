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

/// 写入 .txt 文件（覆盖）
pub fn write_text(path: &Path, content: &str) -> Result<(), StorageError> {
    ensure_dir(path)?;
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
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
