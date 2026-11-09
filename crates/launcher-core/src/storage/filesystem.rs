//! Capability-relative I/O. Every managed path component is opened without following links.
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::core::{LauncherError as Error, Result};

pub fn relative(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 1024 {
        return Err(Error::UnsafePath);
    }
    for part in path.split('/') {
        let base = part.split('.').next().unwrap_or_default().to_uppercase();
        let device = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|p| {
                base.strip_prefix(p).is_some_and(|s| {
                    matches!(
                        s,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || device
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"\\|?*".contains(c))
        {
            return Err(Error::UnsafePath);
        }
    }
    Ok(())
}

pub fn absolute(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path.parent().is_none()
        || path.components().any(|c| matches!(c, Component::ParentDir))
        || path
            .to_str()
            .is_none_or(|s| s.chars().any(char::is_control))
    {
        return Err(Error::UnsafePath);
    }
    Ok(())
}

/// The selected root is an explicit ambient authority. Existing OS parent aliases are
/// canonicalized once; inside the root, only held directory capabilities are used.
pub fn open_root(path: &Path) -> Result<(PathBuf, Dir)> {
    absolute(path)?;
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_dir() || linked(&meta) {
            return Err(Error::UnsafePath);
        }
        let canonical = path.canonicalize()?;
        let dir = Dir::open_ambient_dir(&canonical, ambient_authority())?;
        return Ok((canonical, dir));
    }
    let parent = path.parent().ok_or(Error::UnsafePath)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Error::UnsafePath)?;
    relative(name)?;
    let (parent_path, parent_dir) = open_root(parent)?;
    let dir = directory(&parent_dir, name, true)?;
    Ok((parent_path.join(name), dir))
}

fn linked(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

pub fn directory(parent: &Dir, name: &str, create: bool) -> Result<Dir> {
    relative(name)?;
    // Single component only: intermediate components are never resolved implicitly.
    if name.contains('/') {
        return Err(Error::UnsafePath);
    }
    if create {
        let builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            use cap_std::fs::DirBuilderExt;
            let mut builder = builder;
            builder.mode(0o700);
            builder
        };
        match parent.create_dir_with(name, &builder) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    parent
        .open_dir_nofollow(name)
        .map_err(|_| Error::UnsafePath)
}

pub fn descendants(root: &Dir, path: &str, create: bool) -> Result<Dir> {
    relative(path)?;
    let mut dir = root.try_clone()?;
    for part in path.split('/') {
        dir = directory(&dir, part, create)?;
    }
    Ok(dir)
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

pub fn regular(parent: &Dir, name: &str) -> Result<bool> {
    relative(name)?;
    if name.contains('/') {
        return Err(Error::UnsafePath);
    }
    match parent.symlink_metadata(name) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(Error::UnsafePath),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn read<T: DeserializeOwned>(dir: &Dir, name: &str) -> Result<Option<T>> {
    if !regular(dir, name)? {
        return Ok(None);
    }
    let file = dir.open_with(name, options().read(true))?;
    if !file.metadata()?.is_file() {
        return Err(Error::UnsafePath);
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::CorruptDocument);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| Error::CorruptDocument)?;
    match value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
    {
        Some(1) => {}
        Some(_) => return Err(Error::UnsupportedSchema),
        None => return Err(Error::CorruptDocument),
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|_| Error::CorruptDocument)
}

/// Error::Durability means replacement happened but the parent sync failed. Callers
/// must treat it as committed and refresh in-memory selection even while reporting it.
pub fn write<T: Serialize>(dir: &Dir, name: &str, document: &T) -> Result<()> {
    write_with_publication(dir, name, document, |dir, temporary, name| {
        dir.rename(temporary, dir, name)?;
        sync(dir).map_err(|e| match e {
            Error::Storage(e) => Error::Durability(e),
            e => e,
        })
    })
}

fn write_with_publication<T: Serialize>(
    dir: &Dir,
    name: &str,
    document: &T,
    publish: impl FnOnce(&Dir, &str, &str) -> Result<()>,
) -> Result<()> {
    regular(dir, name)?;
    let bytes = serde_json::to_vec_pretty(document).map_err(|_| Error::CorruptDocument)?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::InvalidInput {
            field: "document size",
        });
    }
    let temporary = format!(".write-{}.tmp", Uuid::new_v4());
    let result = (|| {
        let mut file = dir.open_with(&temporary, options().write(true).create_new(true))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file); // Windows replacement must not retain our temporary handle.
        regular(dir, name)?;
        publish(dir, &temporary, name)
    })();
    if result.is_err() {
        let _ = dir.remove_file(&temporary);
    }
    result
}

pub fn sync(dir: &Dir) -> Result<()> {
    #[cfg(unix)]
    // Linux Dir capabilities may use O_PATH, which cannot be fsynced. Open a
    // readable descriptor for this same directory relative to the held handle.
    // The constant "." introduces no ambient lookup or user-controlled traversal.
    dir.open_with(".", options().read(true))?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

#[derive(Debug)]
pub struct FileLease(std::fs::File);

impl Drop for FileLease {
    fn drop(&mut self) {
        // A concurrent spawn can transiently duplicate the descriptor before exec.
        // Release the lease now even if the kernel still holds such a duplicate.
        let _ = self.0.unlock();
    }
}

pub fn lock(dir: &Dir, name: &str) -> Result<FileLease> {
    regular(dir, name)?;
    let file = dir
        .open_with(name, options().read(true).write(true).create(true))?
        .into_std();
    if !file.metadata()?.is_file() {
        return Err(Error::UnsafePath);
    }
    file.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => Error::StorageBusy,
        std::fs::TryLockError::Error(e) => Error::Storage(e),
    })?;
    Ok(FileLease(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn releasing_a_lease_unlocks_even_with_a_duplicated_descriptor() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (_, dir) = open_root(temp.path())?;
        let lease = lock(&dir, "lease.lock")?;
        let inherited = lease.0.try_clone()?;
        assert!(matches!(lock(&dir, "lease.lock"), Err(Error::StorageBusy)));
        drop(lease);
        let reopened = lock(&dir, "lease.lock")?;
        drop(inherited);
        drop(reopened);
        Ok(())
    }
    #[test]
    fn portable_paths_reject_traversal_windows_devices_and_alternate_streams() {
        for path in [
            "",
            "/absolute",
            "../x",
            "a/../b",
            "a//b",
            "a/./b",
            "C:/x",
            "C:\\x",
            "\\\\server\\share",
            "a:b",
            "CON",
            "aux.txt",
            "LPT1.log",
            "COM¹.txt",
            "a/PRN",
            "a.",
            "a ",
            "a\0b",
            "a\nb",
        ] {
            assert!(relative(path).is_err(), "accepted {path:?}");
        }
        for path in [
            "shared/assets/objects/ab/abcdef",
            "profiles/мир с пробелами",
            "ordinary/config.json",
        ] {
            assert!(relative(path).is_ok());
        }
    }
    #[test]
    fn failed_replacement_preserves_existing_document()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::TempDir::new()?;
        let (_, dir) = open_root(temp.path())?;
        write(
            &dir,
            "test.json",
            &serde_json::json!({"schemaVersion":1,"value":"old"}),
        )?;
        let before = std::fs::read(temp.path().join("test.json"))?;
        let huge = "x".repeat(1024 * 1024 + 1);
        assert!(
            write(
                &dir,
                "test.json",
                &serde_json::json!({"schemaVersion":1,"value":huge})
            )
            .is_err()
        );
        assert_eq!(std::fs::read(temp.path().join("test.json"))?, before);
        assert_eq!(dir.entries()?.count(), 1);
        Ok(())
    }
    #[test]
    fn interrupted_publication_preserves_old_json_and_reports_post_commit_failure()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::TempDir::new()?;
        let (_, dir) = open_root(temp.path())?;
        let old = serde_json::json!({"schemaVersion":1,"value":"old"});
        let new = serde_json::json!({"schemaVersion":1,"value":"new"});
        write(&dir, "test.json", &old)?;
        let failed = write_with_publication(&dir, "test.json", &new, |_, _, _| {
            Err(std::io::Error::other("injected rename failure").into())
        });
        assert!(failed.is_err());
        assert_eq!(read::<serde_json::Value>(&dir, "test.json")?, Some(old));
        assert_eq!(dir.entries()?.count(), 1);
        let committed = write_with_publication(&dir, "test.json", &new, |dir, temp, name| {
            dir.rename(temp, dir, name)?;
            Err(Error::Durability(std::io::Error::other(
                "injected parent sync failure",
            )))
        });
        assert!(matches!(committed, Err(Error::Durability(_))));
        assert_eq!(read::<serde_json::Value>(&dir, "test.json")?, Some(new));
        assert_eq!(dir.entries()?.count(), 1);
        Ok(())
    }
}
