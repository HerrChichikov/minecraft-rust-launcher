use super::PreparedLaunch;
use crate::{
    application,
    auth::LaunchIdentity,
    core::{LauncherError as Error, OperatingSystem, Result},
    installer::metadata,
    runtime::execution_path,
};
use std::{borrow::Cow, collections::BTreeMap, ffi::OsString, path::Path};

fn path_value(path: &Path) -> Result<String> {
    execution_path(path)?
        .into_os_string()
        .into_string()
        .map_err(|_| Error::UnsafePath)
}

pub(super) fn build(launch: &PreparedLaunch, identity: &LaunchIdentity) -> Result<Vec<OsString>> {
    let separator = if launch.context.platform.os == OperatingSystem::Windows {
        ";"
    } else {
        ":"
    };
    let classpath = classpath(&launch.installed.classpath, separator)?;
    let cwd = path_value(&launch.working_directory)?;
    let values: BTreeMap<&str, Cow<'_, str>> = BTreeMap::from([
        ("auth_player_name", identity.profile.name.as_str().into()),
        ("auth_uuid", identity.profile.id.simple().to_string().into()),
        (
            "auth_access_token",
            identity.access_token.expose_secret().into(),
        ),
        ("user_type", identity.user_type.as_str().into()),
        ("clientid", "".into()),
        ("auth_xuid", "".into()),
        (
            "version_name",
            launch.profile.minecraft_version.as_str().into(),
        ),
        (
            "version_type",
            metadata::field(&launch.plan.metadata, "type")?.into(),
        ),
        ("game_directory", cwd.into()),
        (
            "assets_root",
            path_value(&launch.store.path("shared/assets")?)?.into(),
        ),
        (
            "assets_index_name",
            launch.installed.asset_index_id.as_str().into(),
        ),
        (
            "natives_directory",
            path_value(&launch.natives_directory)?.into(),
        ),
        ("launcher_name", "Launcher".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath_separator", separator.into()),
        ("classpath", classpath.into()),
    ]);
    if launch.profile.minecraft_version != application::TARGET_MINECRAFT_VERSION {
        return Err(Error::Metadata);
    }

    let settings = &launch.profile.settings;
    let mut args = vec![
        OsString::from(format!("-Xms{}M", settings.min_ram_mib)),
        OsString::from(format!("-Xmx{}M", settings.max_ram_mib)),
        OsString::from("-XX:-CreateCoredumpOnCrash"),
        OsString::from("-XX:+SuppressFatalErrorMessage"),
        OsString::from(format!(
            "-Dlog4j.configurationFile={}",
            path_value(&launch.logging_config)?
        )),
    ];
    args.extend(settings.extra_jvm_args.iter().map(OsString::from));
    let arguments = &launch.plan.metadata["arguments"];
    args.extend(expand(&arguments["jvm"], &launch.context, &values)?);
    args.push(launch.plan.main_class.clone().into());
    args.extend(expand(&arguments["game"], &launch.context, &values)?);
    Ok(args)
}

fn classpath(entries: &[String], separator: &str) -> Result<String> {
    if entries.is_empty() {
        return Err(Error::InvalidLaunchArguments);
    }
    let mut paths = Vec::with_capacity(entries.len());
    for entry in entries {
        crate::storage::filesystem::relative(entry)?;
        if !entry.starts_with("shared/") || entry.contains(separator) {
            return Err(Error::InvalidLaunchArguments);
        }
        // The fixed working directory is profiles/<id>/game. Relative entries avoid
        // repeating the data root in every entry of the Win32 command line.
        paths.push(format!("../../../{entry}"));
    }
    Ok(paths.join(separator))
}

fn expand(
    args: &serde_json::Value,
    context: &metadata::RuleContext,
    values: &BTreeMap<&str, Cow<'_, str>>,
) -> Result<Vec<OsString>> {
    metadata::argument_values(args, context)?
        .iter()
        .map(|arg| substitute(arg, values).map(OsString::from))
        .collect()
}

fn substitute(template: &str, values: &BTreeMap<&str, Cow<'_, str>>) -> Result<String> {
    let mut expanded = String::new();
    let mut remaining = template;
    while let Some((prefix, variable)) = remaining.split_once("${") {
        let (name, suffix) = variable
            .split_once('}')
            .ok_or(Error::InvalidLaunchArguments)?;
        expanded.push_str(prefix);
        expanded.push_str(values.get(name).ok_or(Error::InvalidLaunchArguments)?);
        remaining = suffix;
    }
    expanded.push_str(remaining);
    if expanded.contains('\0') {
        return Err(Error::InvalidLaunchArguments);
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests;
