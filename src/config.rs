//! On-disk state under `~/.talbot`.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, ensure};

const MAX_UPDATE_OFFSET_AGE: Duration = Duration::from_secs(6 * 24 * 60 * 60);

pub(crate) fn dir() -> Result<PathBuf> {
    let home = std::env::home_dir().context("cannot determine home directory")?;
    Ok(home.join(".talbot"))
}

pub fn read_token() -> Result<String> {
    let path = dir()?.join("token");
    let token =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let token = token.trim();
    ensure!(!token.is_empty(), "{} is empty", path.display());
    Ok(token.to_string())
}

pub fn write_token(token: &str) -> Result<PathBuf> {
    let path = write("token", token)?;
    clear_update_offset()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

pub fn read_chat_id() -> Option<String> {
    let id = fs::read_to_string(dir().ok()?.join("chat_id")).ok()?;
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_string())
}

pub fn write_chat_id(id: &str) -> Result<PathBuf> {
    write("chat_id", id)
}

pub fn read_update_offset() -> Option<i64> {
    let path = dir().ok()?.join("update_offset");
    let age = fs::metadata(&path).ok()?.modified().ok()?.elapsed().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    parse_update_offset(&contents, age)
}

pub fn write_update_offset(offset: i64) -> Result<PathBuf> {
    write("update_offset", &offset.to_string())
}

fn clear_update_offset() -> Result<()> {
    let path = dir()?.join("update_offset");
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("cannot remove {}", path.display())),
    }
}

fn parse_update_offset(contents: &str, age: Duration) -> Option<i64> {
    (age <= MAX_UPDATE_OFFSET_AGE)
        .then(|| contents.trim().parse().ok())
        .flatten()
}

fn write(name: &str, contents: &str) -> Result<PathBuf> {
    let dir = dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = dir.join(name);
    fs::write(&path, contents).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_recent_valid_telegram_update_offsets() {
        assert_eq!(
            parse_update_offset("42\n", Duration::from_secs(60)),
            Some(42)
        );
        assert_eq!(parse_update_offset("not-a-number", Duration::ZERO), None);
        assert_eq!(
            parse_update_offset("42", MAX_UPDATE_OFFSET_AGE + Duration::from_secs(1)),
            None
        );
    }
}
