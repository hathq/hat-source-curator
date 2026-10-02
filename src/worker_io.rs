use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn arguments() -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let key = flag
            .strip_prefix("--")
            .ok_or("worker arguments must be named")?;
        let value = args.next().ok_or("worker argument value is missing")?;
        if values.insert(key.to_owned(), value).is_some() {
            return Err("worker argument is duplicated".into());
        }
    }
    Ok(values)
}

pub(super) fn required<'a>(
    values: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("--{key} is required"))
}

pub(super) fn common_args(
    values: &BTreeMap<String, String>,
    repository_id: &str,
) -> Result<Vec<String>, String> {
    Ok(vec![
        "--context-partition-id".into(),
        required(values, "context-partition-id")?.into(),
        "--repository-id".into(),
        repository_id.into(),
    ])
}

pub(super) fn run_hatter(
    values: &BTreeMap<String, String>,
    command: &str,
    common: &[String],
    extra: &[&str],
) -> Result<Vec<u8>, String> {
    let output = Command::new(required(values, "hatter")?)
        .env("HATTER_HOME", required(values, "hatter-home")?)
        .arg("hat")
        .arg(command)
        .args(common)
        .args(extra)
        .output()
        .map_err(message)?;
    if !output.status.success() {
        return Err(format!(
            "hatter {command} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

pub(super) fn canonical_directory(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    let metadata = fs::symlink_metadata(path).map_err(message)?;
    let canonical = fs::canonicalize(path).map_err(message)?;
    if !path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || canonical != path
    {
        return Err("worker directory must be one exact absolute directory".into());
    }
    Ok(canonical)
}

pub(super) fn write_document(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > 1_048_576 {
        return Err("worker document exceeds 1 MiB".into());
    }
    if path.exists() {
        return if fs::read(path).map_err(message)? == bytes {
            Ok(())
        } else {
            Err("worker document already differs".into())
        };
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(message)?;
    file.write_all(bytes).map_err(message)?;
    file.sync_all().map_err(message)
}

pub(super) fn path(value: &Path) -> Result<&str, String> {
    value
        .to_str()
        .ok_or_else(|| "worker path is not UTF-8".into())
}

pub(super) fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}
