//! Installation lease and capability-backed resource publication.
use super::{ProfileLease, WorkspaceService, filesystem as fs};
use crate::{
    core::{LauncherError as Error, Result},
    installer::{Cancellation, metadata::Resource},
    profiles::ProfileId,
};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use sha1::{Digest, Sha1};
use std::{
    io::{Read, Write},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct ResourceStore {
    dir: Dir,
    path: std::path::PathBuf,
    pub root_id: Uuid,
    _profile: ProfileLease,
    _installation: std::fs::File,
}
impl WorkspaceService {
    pub fn installation_store(
        &self,
        root_id: Uuid,
        profile_id: ProfileId,
    ) -> Result<Arc<ResourceStore>> {
        if self.snapshot()?.root_id != root_id {
            return Err(Error::RevisionConflict);
        }
        let profile = self.lease_profile(profile_id)?;
        self.load_verification_index()?;
        let dir = self.root.dir.try_clone()?;
        let installation = fs::lock(
            &fs::descendants(&dir, "state/locks", false)?,
            "installation.lock",
        )?;
        Ok(Arc::new(ResourceStore {
            dir,
            path: self.snapshot()?.data_root,
            root_id,
            _profile: profile,
            _installation: installation,
        }))
    }
}
impl ResourceStore {
    pub(crate) fn bounded_bytes(&self, path: &str, limit: u64) -> Result<Option<Vec<u8>>> {
        let (dir, name) = self.parent(path, true)?;
        if !fs::regular(&dir, &name)? {
            return Ok(None);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = dir.open_with(name, &options)?;
        if file.metadata()?.len() > limit {
            return Ok(None);
        }
        let mut bytes = vec![];
        file.take(limit + 1).read_to_end(&mut bytes)?;
        Ok((bytes.len() as u64 <= limit).then_some(bytes))
    }
    fn parent(&self, path: &str, create: bool) -> Result<(Dir, String)> {
        fs::relative(path)?;
        let (parent, name) = path.rsplit_once('/').ok_or(Error::UnsafePath)?;
        Ok((fs::descendants(&self.dir, parent, create)?, name.into()))
    }
    pub fn path(&self, relative: &str) -> Result<std::path::PathBuf> {
        fs::relative(relative)?;
        Ok(self.path.join(relative))
    }
    pub fn valid(&self, resource: &Resource, cancel: &Cancellation) -> Result<bool> {
        Resource::new(resource.path.clone(), resource.download.clone())?;
        let (dir, name) = self.parent(&resource.path, true)?;
        if !fs::regular(&dir, &name)? {
            return Ok(false);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut file = dir.open_with(&name, &options)?;
        if file.metadata()?.len() != resource.download.size {
            return Ok(false);
        }
        let mut hash = Sha1::new();
        let mut buf = [0; 65536];
        loop {
            cancel.check()?;
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
        }
        Ok(format!("{:x}", hash.finalize()) == resource.download.sha1)
    }
    pub fn publish(
        &self,
        resource: &Resource,
        mut input: impl Read,
        cancel: &Cancellation,
        mut progress: impl FnMut(u64),
    ) -> Result<()> {
        Resource::new(resource.path.clone(), resource.download.clone())?;
        let (dir, name) = self.parent(&resource.path, true)?;
        fs::regular(&dir, &name)?;
        let temp = format!(".download-{}.part", Uuid::new_v4());
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut file = dir.open_with(&temp, &options)?;
            let mut hash = Sha1::new();
            let mut size = 0u64;
            let mut buf = [0; 65536];
            loop {
                cancel.check()?;
                let n = input.read(&mut buf).map_err(|_| Error::Network)?;
                if n == 0 {
                    break;
                }
                size = size.checked_add(n as u64).ok_or(Error::Integrity)?;
                if size > resource.download.size {
                    return Err(Error::Integrity);
                }
                file.write_all(&buf[..n])?;
                hash.update(&buf[..n]);
                progress(n as u64);
            }
            if size != resource.download.size
                || format!("{:x}", hash.finalize()) != resource.download.sha1
            {
                return Err(Error::Integrity);
            }
            file.sync_all()?;
            drop(file);
            cancel.check()?;
            fs::regular(&dir, &name)?;
            dir.rename(&temp, &dir, &name)?;
            fs::sync(&dir)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = dir.remove_file(&temp);
        }
        result
    }
    pub fn read_json(&self, path: &str) -> Result<serde_json::Value> {
        let (dir, name) = self.parent(path, false)?;
        if !fs::regular(&dir, &name)? {
            return Err(Error::CorruptDocument);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = dir.open_with(name, &options)?;
        let mut bytes = vec![];
        file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Metadata);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Metadata)
    }
    pub fn document(&self, path: &str, value: &impl serde::Serialize) -> Result<()> {
        let (dir, name) = self.parent(path, true)?;
        fs::write(&dir, &name, value)
    }
}

impl ResourceStore {
    pub(crate) fn directory(&self, path: &str) -> Result<()> {
        fs::descendants(&self.dir, path, true)?;
        Ok(())
    }
    pub(crate) fn executable(&self, path: &str) -> Result<()> {
        let (dir, name) = self.parent(path, false)?;
        if !fs::regular(&dir, &name)? {
            return Err(Error::UnsafePath);
        }
        #[cfg(unix)]
        {
            use cap_std::fs::PermissionsExt;
            dir.set_permissions(name, cap_std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }
    pub(crate) fn copy(&self, source: &str, target: &str) -> Result<()> {
        let (from, name) = self.parent(source, false)?;
        if !fs::regular(&from, &name)? {
            return Err(Error::Integrity);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut input = from.open_with(name, &options)?;
        let (to, name) = self.parent(target, true)?;
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let mut output = to.open_with(name, &options)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        Ok(())
    }
    pub(crate) fn link(&self, link: &str, target: &str) -> Result<()> {
        let (dir, name) = self.parent(link, true)?;
        #[cfg(unix)]
        {
            dir.symlink(target, name)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = (dir, name, target);
            Err(Error::Metadata)
        }
    }

    pub(crate) fn publish_directory(&self, staging: &str, target: &str) -> Result<()> {
        let (from, name) = self.parent(staging, false)?;
        let (to, target) = self.parent(target, true)?;
        from.rename(name, &to, target)?;
        fs::sync(&to)
    }
}

impl ResourceStore {
    pub(crate) fn link_matches(&self, path: &str, target: &str) -> Result<bool> {
        let (dir, name) = self.parent(path, false)?;
        let metadata = dir.symlink_metadata(&name)?;
        Ok(metadata.file_type().is_symlink()
            && dir.read_link(name)? == std::path::Path::new(target))
    }
}

impl ResourceStore {
    pub(crate) fn directory_valid(&self, path: &str) -> Result<bool> {
        fs::relative(path)?;
        let mut dir = self.dir.try_clone()?;
        for component in path.split('/') {
            match dir.symlink_metadata(component) {
                Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {
                    dir = fs::directory(&dir, component, false)?;
                }
                Ok(_) => return Err(Error::UnsafePath),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(true)
    }
}
