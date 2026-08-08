//! On-disk state: `~/.talbot/token` and `~/.talbot/chat_id`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};

fn dir() -> Result<PathBuf> {
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

fn write(name: &str, contents: &str) -> Result<PathBuf> {
    let dir = dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = dir.join(name);
    fs::write(&path, contents).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(path)
}
