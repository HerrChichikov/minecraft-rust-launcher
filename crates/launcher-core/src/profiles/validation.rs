use super::{LaunchSettings, Profile, ProfileId};
use crate::{
    core::{LauncherError as Error, Result},
    runtime::RuntimeSelection,
    storage::filesystem,
};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPolicy {
    pub max_ram_mib: u32,
}

impl SettingsPolicy {
    pub fn new(physical_ram_mib: u64) -> Result<Self> {
        if physical_ram_mib < 256 {
            return Err(Error::InvalidInput {
                field: "system memory",
            });
        }
        Ok(Self {
            max_ram_mib: physical_ram_mib.min(1_048_576) as u32,
        })
    }
    pub fn defaults(self) -> LaunchSettings {
        let max = self.max_ram_mib.min(4096);
        LaunchSettings {
            min_ram_mib: max.min(1024),
            max_ram_mib: max,
            runtime: RuntimeSelection::Managed,
            extra_jvm_args: vec![],
        }
    }
}

pub fn id(id: ProfileId) -> Result<()> {
    if id.0.get_version_num() != 4 || id.0.get_variant() != uuid::Variant::RFC4122 {
        return Err(Error::InvalidInput {
            field: "profile ID",
        });
    }
    Ok(())
}

pub fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 64 || value.chars().any(char::is_control) {
        return Err(Error::InvalidInput {
            field: "profile name",
        });
    }
    Ok(value.into())
}

pub fn profile(profile: &Profile) -> Result<()> {
    id(profile.id)?;
    if profile.schema_version != 1 {
        return Err(Error::UnsupportedSchema);
    }
    if profile.revision == 0
        || name(&profile.name)? != profile.name
        || profile.minecraft_version != "1.21.1"
    {
        return Err(Error::CorruptDocument);
    }
    settings(
        &profile.settings,
        SettingsPolicy {
            max_ram_mib: 1_048_576,
        },
        false,
    )
}

pub fn settings(
    value: &LaunchSettings,
    policy: SettingsPolicy,
    check_executable: bool,
) -> Result<()> {
    if value.min_ram_mib < 256
        || value.min_ram_mib > value.max_ram_mib
        || value.max_ram_mib > policy.max_ram_mib
    {
        return Err(Error::InvalidInput { field: "RAM" });
    }
    if value.extra_jvm_args.len() > 32 || value.extra_jvm_args.iter().any(|a| !jvm_arg(a)) {
        return Err(Error::InvalidInput {
            field: "JVM arguments",
        });
    }
    if let RuntimeSelection::Custom { executable } = &value.runtime {
        filesystem::absolute(executable)?;
        if cfg!(windows)
            && !executable
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
        {
            return Err(Error::InvalidInput {
                field: "Java executable",
            });
        }
        if check_executable {
            executable_file(executable)?;
        }
    }
    Ok(())
}

fn executable_file(path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path).map_err(|_| Error::InvalidInput {
        field: "Java executable",
    })?;
    let mut executable = meta.is_file();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        executable &= meta.permissions().mode() & 0o111 != 0;
    }
    if !executable {
        return Err(Error::InvalidInput {
            field: "Java executable",
        });
    }
    Ok(())
}

fn jvm_arg(value: &str) -> bool {
    if value.len() > 512 || value.chars().any(char::is_control) {
        return false;
    }
    matches!(
        value,
        "-XX:+UseG1GC" | "-XX:+UseZGC" | "-XX:+UseParallelGC" | "-XX:+DisableExplicitGC"
    ) || value
        .strip_prefix("-XX:MaxGCPauseMillis=")
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| (1..=10_000).contains(&n))
}
