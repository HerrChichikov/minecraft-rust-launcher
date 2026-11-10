use super::{AppPaths, filesystem as fs};
use crate::{
    core::{LauncherError as Error, Result},
    profiles::{LaunchSettings, Profile, ProfileId, ProfileRepository, SettingsPolicy, validation},
};
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bootstrap {
    schema_version: u32,
    revision: u32,
    data_root: PathBuf,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Marker {
    schema_version: u32,
    root_id: Uuid,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    schema_version: u32,
    root_id: Uuid,
    revision: u32,
    active_profile_id: ProfileId,
}

#[derive(Debug)]
pub(super) struct Root {
    pub(super) id: Uuid,
    path: PathBuf,
    pub(super) dir: Dir,
    _lock: fs::FileLease,
    policy: SettingsPolicy,
}

#[derive(Debug)]
pub struct ProfileLease {
    _root: Arc<Root>,
    _lock: fs::FileLease,
    pub profile_id: ProfileId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub schema_version: u32,
    pub root_id: Uuid,
    pub data_root: PathBuf,
    pub configuration_revision: u32,
    pub revision: u32,
    pub active_profile_id: ProfileId,
    pub installation_recorded: bool,
    pub profiles: Vec<Profile>,
    pub policy: SettingsPolicy,
}

/// Owns OS leases for both the stable configuration location and selected root.
/// Its short JSON operations run on a desktop blocking worker, never a UI thread.
#[derive(Debug)]
pub struct WorkspaceService {
    config: Dir,
    config_path: PathBuf,
    _app_lock: fs::FileLease,
    bootstrap: Bootstrap,
    pub(super) root: Arc<Root>,
    policy: SettingsPolicy,
}

impl WorkspaceService {
    pub fn open(paths: AppPaths, policy: SettingsPolicy) -> Result<Self> {
        let policy = SettingsPolicy::new(u64::from(policy.max_ram_mib))?;
        let (config_path, config) = fs::open_root(&paths.configuration)?;
        let app_lock = fs::lock(&config, ".launcher.lock")?;
        let loaded: Option<Bootstrap> = fs::read(&config, "launcher.json")?;
        let bootstrap = loaded.unwrap_or(Bootstrap {
            schema_version: 1,
            revision: 1,
            data_root: paths.data,
        });
        if bootstrap.revision == 0 {
            return Err(Error::CorruptDocument);
        }
        if config_path.starts_with(prospective(&bootstrap.data_root)?) {
            return Err(Error::UnsafePath);
        }
        let root = Arc::new(Root::open(&bootstrap.data_root, policy)?);
        let bootstrap = Bootstrap {
            data_root: root.path.clone(),
            ..bootstrap
        };
        if !fs::regular(&config, "launcher.json")? {
            fs::write(&config, "launcher.json", &bootstrap)?;
        }
        let service = Self {
            config,
            config_path,
            _app_lock: app_lock,
            bootstrap,
            root,
            policy,
        };
        service.snapshot()?;
        Ok(service)
    }

    pub fn snapshot(&self) -> Result<WorkspaceSnapshot> {
        let selection = self.root.selection()?;
        let profiles = self.root.list()?;
        if !profiles.iter().any(|p| p.id == selection.active_profile_id) {
            return Err(Error::CorruptDocument);
        }
        let active_id = selection.active_profile_id.0.to_string();
        let receipt = format!("installation-{active_id}.json");
        let installation_recorded = fs::read::<serde_json::Value>(&self.root.state()?, &receipt)
            .ok()
            .flatten()
            .is_some_and(|value| value["profileId"].as_str() == Some(active_id.as_str()));
        Ok(WorkspaceSnapshot {
            schema_version: 1,
            root_id: self.root.id,
            data_root: self.root.path.clone(),
            configuration_revision: self.bootstrap.revision,
            revision: selection.revision,
            active_profile_id: selection.active_profile_id,
            installation_recorded,
            profiles,
            policy: self.policy,
        })
    }

    fn context(&self, root_id: Uuid) -> Result<()> {
        if root_id != self.root.id {
            return Err(Error::RevisionConflict);
        }
        Ok(())
    }

    pub fn create_profile(&self, root_id: Uuid, name: &str) -> Result<WorkspaceSnapshot> {
        self.context(root_id)?;
        self.root.create(name)?;
        self.snapshot()
    }

    pub fn save_profile(
        &self,
        root_id: Uuid,
        id: ProfileId,
        expected_revision: u32,
        name: &str,
        settings: LaunchSettings,
    ) -> Result<WorkspaceSnapshot> {
        self.context(root_id)?;
        let mut profile = self.root.load(id)?;
        if profile.revision != expected_revision {
            return Err(Error::RevisionConflict);
        }
        profile.name = validation::name(name)?;
        validation::settings(&settings, self.policy, true)?;
        profile.settings = settings;
        self.root.save(&profile)?;
        self.snapshot()
    }

    pub fn select_profile(
        &self,
        root_id: Uuid,
        id: ProfileId,
        expected_revision: u32,
    ) -> Result<WorkspaceSnapshot> {
        self.context(root_id)?;
        let _lease = self.lease_profile(id)?;
        self.root.load(id)?;
        let mut selection = self.root.selection()?;
        if selection.revision != expected_revision {
            return Err(Error::RevisionConflict);
        }
        selection.revision = next(selection.revision)?;
        selection.active_profile_id = id;
        fs::write(&self.root.state()?, "root-settings.json", &selection)?;
        self.snapshot()
    }

    pub fn lease_profile(&self, id: ProfileId) -> Result<ProfileLease> {
        self.root.load(id)?;
        let lock = self.root.profile_lock(id)?;
        Ok(ProfileLease {
            _root: Arc::clone(&self.root),
            _lock: lock,
            profile_id: id,
        })
    }

    pub fn profile_game_dir(&self, id: ProfileId) -> Result<PathBuf> {
        self.root.load(id)?;
        fs::descendants(&self.root.dir, &format!("profiles/{}/game", id.0), false)?;
        Ok(self
            .root
            .path
            .join("profiles")
            .join(id.0.to_string())
            .join("game"))
    }

    pub fn switch_root(
        &mut self,
        root_id: Uuid,
        expected_revision: u32,
        path: &Path,
    ) -> Result<WorkspaceSnapshot> {
        self.context(root_id)?;
        if self.bootstrap.revision != expected_revision {
            return Err(Error::RevisionConflict);
        }
        if Arc::strong_count(&self.root) != 1 {
            return Err(Error::ProfileBusy);
        }
        fs::absolute(path)?;
        // Resolve the nearest existing ancestor before any writes, including OS parent aliases.
        let candidate = prospective(path)?;
        if candidate == self.root.path {
            return self.snapshot();
        }
        if candidate.starts_with(&self.root.path)
            || self.root.path.starts_with(&candidate)
            || self.config_path.starts_with(&candidate)
        {
            return Err(Error::UnsafePath);
        }
        let root = Arc::new(Root::open(&candidate, self.policy)?);
        let bootstrap = Bootstrap {
            schema_version: 1,
            revision: next(self.bootstrap.revision)?,
            data_root: root.path.clone(),
        };
        let result = fs::write(&self.config, "launcher.json", &bootstrap);
        if result.is_ok() || matches!(&result, Err(Error::Durability(_))) {
            self.root = root;
            self.bootstrap = bootstrap;
        }
        result?;
        self.snapshot()
    }
}

fn prospective(path: &Path) -> Result<PathBuf> {
    fs::absolute(path)?;
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(Error::UnsafePath);
        }
        return Ok(path.canonicalize()?);
    }
    let parent = prospective(path.parent().ok_or(Error::UnsafePath)?)?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Error::UnsafePath)?;
    fs::relative(name)?;
    Ok(parent.join(name))
}

fn next(revision: u32) -> Result<u32> {
    revision.checked_add(1).ok_or(Error::RevisionConflict)
}

impl Root {
    fn open(path: &Path, policy: SettingsPolicy) -> Result<Self> {
        let (path, dir) = fs::open_root(path)?;
        let marker: Option<Marker> = fs::read(&dir, "root.json")?;
        if marker.is_none() {
            for entry in dir.entries()? {
                let entry = entry?;
                if entry.file_name() != ".launcher-root.lock" {
                    return Err(Error::UnmanagedDirectory);
                }
            }
        }
        let lock = fs::lock(&dir, ".launcher-root.lock")?;
        // Re-read after acquiring the OS lock; another launcher may have initialized it.
        let marker: Option<Marker> = fs::read(&dir, "root.json")?;
        let marker = match marker {
            Some(marker)
                if marker.root_id.get_version_num() == 4
                    && marker.root_id.get_variant() == uuid::Variant::RFC4122 =>
            {
                marker
            }
            Some(_) => return Err(Error::CorruptDocument),
            None => {
                let marker = Marker {
                    schema_version: 1,
                    root_id: Uuid::new_v4(),
                };
                fs::write(&dir, "root.json", &marker)?;
                marker
            }
        };
        for path in [
            "profiles",
            "state/locks",
            "shared/metadata",
            "shared/versions",
            "shared/libraries",
            "shared/assets/indexes",
            "shared/assets/objects",
            "shared/runtimes",
            "shared/agents",
        ] {
            fs::descendants(&dir, path, true)?;
        }
        let root = Self {
            id: marker.root_id,
            path,
            dir,
            _lock: lock,
            policy,
        };
        let state = root.state()?;
        if fs::read::<Selection>(&state, "root-settings.json")?.is_none() {
            let profiles = root.list()?;
            let profile = match profiles.into_iter().next() {
                Some(p) => p,
                None => root.create("Основная конфигурация")?,
            };
            fs::write(
                &state,
                "root-settings.json",
                &Selection {
                    schema_version: 1,
                    root_id: root.id,
                    revision: 1,
                    active_profile_id: profile.id,
                },
            )?;
        }
        let selection = root.selection()?;
        if !root
            .list()?
            .iter()
            .any(|p| p.id == selection.active_profile_id)
        {
            return Err(Error::CorruptDocument);
        }
        Ok(root)
    }

    pub(super) fn state(&self) -> Result<Dir> {
        fs::directory(&self.dir, "state", false)
    }
    fn profiles(&self) -> Result<Dir> {
        fs::directory(&self.dir, "profiles", false)
    }
    fn selection(&self) -> Result<Selection> {
        let selection: Selection =
            fs::read(&self.state()?, "root-settings.json")?.ok_or(Error::CorruptDocument)?;
        if selection.root_id != self.id || selection.revision == 0 {
            return Err(Error::CorruptDocument);
        }
        validation::id(selection.active_profile_id)?;
        Ok(selection)
    }
    fn profile_lock(&self, id: ProfileId) -> Result<fs::FileLease> {
        validation::id(id)?;
        fs::lock(
            &fs::descendants(&self.dir, "state/locks", false)?,
            &format!("{}.lock", id.0),
        )
        .map_err(|e| match e {
            Error::StorageBusy => Error::ProfileBusy,
            e => e,
        })
    }
    fn profile_dir(&self, id: ProfileId) -> Result<Dir> {
        validation::id(id)?;
        fs::directory(&self.profiles()?, &id.0.to_string(), false)
    }
}

impl ProfileRepository for Root {
    fn load(&self, id: ProfileId) -> Result<Profile> {
        let dir = self.profile_dir(id)?;
        // Check every mutable directory even when the current operation only edits JSON.
        for name in ["game", "logs", "runs"] {
            fs::directory(&dir, name, false)?;
        }
        let profile: Profile = fs::read(&dir, "config.json")?.ok_or(Error::CorruptDocument)?;
        validation::profile(&profile)?;
        if profile.id != id {
            return Err(Error::CorruptDocument);
        }
        Ok(profile)
    }
    fn list(&self) -> Result<Vec<Profile>> {
        let mut profiles = vec![];
        let mut entries = 0;
        for entry in self.profiles()?.entries()? {
            entries += 1;
            if entries > 512 {
                return Err(Error::InvalidInput {
                    field: "profile count",
                });
            }
            let name = entry?.file_name();
            let Some(name) = name.to_str() else {
                return Err(Error::UnsafePath);
            };
            if name.starts_with(".creating-") {
                continue;
            }
            if let Ok(id) = Uuid::parse_str(name) {
                if id.to_string() != name {
                    return Err(Error::UnsafePath);
                }
                profiles.push(self.load(ProfileId(id))?);
            }
        }
        profiles.sort_by_key(|p| p.id.0);
        Ok(profiles)
    }
    fn create(&self, name: &str) -> Result<Profile> {
        let name = validation::name(name)?;
        if self.list()?.len() >= 128 {
            return Err(Error::InvalidInput {
                field: "profile count",
            });
        }
        let profile = Profile {
            schema_version: 1,
            revision: 1,
            id: ProfileId(Uuid::new_v4()),
            name,
            minecraft_version: "1.21.1".into(),
            settings: self.policy.defaults(),
        };
        let parent = self.profiles()?;
        let staging = format!(".creating-{}", profile.id.0);
        let dir = fs::directory(&parent, &staging, true)?;
        for name in ["game", "logs", "runs"] {
            fs::directory(&dir, name, true)?;
        }
        fs::write(&dir, "config.json", &profile)?;
        fs::sync(&dir)?;
        drop(dir);
        parent.rename(&staging, &parent, profile.id.0.to_string())?;
        fs::sync(&parent)?;
        Ok(profile)
    }
    fn save(&self, profile: &Profile) -> Result<()> {
        validation::profile(profile)?;
        validation::settings(&profile.settings, self.policy, true)?;
        let _lock = self.profile_lock(profile.id)?;
        let previous = self.load(profile.id)?;
        if previous.revision != profile.revision {
            return Err(Error::RevisionConflict);
        }
        let updated = Profile {
            revision: next(previous.revision)?,
            ..profile.clone()
        };
        fs::write(&self.profile_dir(profile.id)?, "config.json", &updated)
    }
}
