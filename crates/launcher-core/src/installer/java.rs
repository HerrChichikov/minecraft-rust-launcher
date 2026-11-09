use super::{
    Cancellation,
    engine::InstallEngine,
    metadata::{self, Download, Resource},
};
use crate::{
    core::{Architecture, LauncherError as Error, OperatingSystem, Platform, Result},
    runtime::RuntimeSelection,
    storage::filesystem,
};
use serde_json::Value;
use std::{collections::BTreeMap, process::Stdio, time::Duration};
const CATALOG: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

pub(crate) fn platform_key(platform: Platform) -> &'static str {
    match (platform.os, platform.architecture) {
        (OperatingSystem::Windows, _) => "windows-x64",
        (OperatingSystem::Linux, _) => "linux",
        (OperatingSystem::Macos, Architecture::X86_64) => "mac-os",
        (OperatingSystem::Macos, Architecture::Aarch64) => "mac-os-arm64",
    }
}
pub(crate) fn normalized_link(path: &str, target: &str) -> Result<String> {
    if target.starts_with('/') || target.contains(['\\', ':', '\0']) {
        return Err(Error::UnsafePath);
    }
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    for part in target.split('/') {
        match part {
            ".." => {
                parts.pop().ok_or(Error::UnsafePath)?;
            }
            "." => {}
            "" => return Err(Error::UnsafePath),
            part => {
                filesystem::relative(part)?;
                parts.push(part);
            }
        }
    }
    if parts.is_empty() {
        return Err(Error::UnsafePath);
    }
    Ok(parts.join("/"))
}
fn links_valid(files: &serde_json::Map<String, Value>) -> Result<()> {
    for (path, value) in files {
        filesystem::relative(path)?;
        if metadata::field(value, "type")? == "link" {
            let mut next = path.clone();
            let mut visited = std::collections::BTreeSet::new();
            loop {
                if !visited.insert(next.clone()) {
                    return Err(Error::UnsafePath);
                }
                let entry = files.get(&next).ok_or(Error::Metadata)?;
                if metadata::field(entry, "type")? != "link" {
                    if metadata::field(entry, "type")? != "file" {
                        return Err(Error::Metadata);
                    }
                    break;
                }
                next = normalized_link(&next, metadata::field(entry, "target")?)?;
            }
        }
        // A manifest cannot place files or directories beneath a link.
        let mut ancestor = path.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if files.get(parent).is_some_and(|v| v["type"] != "directory") {
                return Err(Error::UnsafePath);
            }
            ancestor = parent;
        }
    }
    Ok(())
}
pub(crate) fn install(
    engine: &InstallEngine,
    component: &str,
    major: u32,
    platform: Platform,
    selection: &RuntimeSelection,
) -> Result<String> {
    if let RuntimeSelection::Custom { executable } = selection {
        filesystem::absolute(executable)?;
        probe(executable, major, platform, &engine.cancel)?;
        return Ok(crate::runtime::execution_path(executable)?
            .to_str()
            .ok_or(Error::UnsafePath)?
            .into());
    }
    let (catalog, _) = engine.http.json(CATALOG, None, &engine.cancel)?;
    let candidates = catalog[platform_key(platform)][component]
        .as_array()
        .ok_or(Error::RuntimeIncompatible)?;
    let candidate = candidates
        .iter()
        .filter(|v| v["availability"]["progress"] == 100)
        .max_by_key(|v| v["version"]["released"].as_str())
        .ok_or(Error::RuntimeIncompatible)?;
    let descriptor: Download = metadata::decode(&candidate["manifest"])?;
    descriptor.validate()?;
    let (manifest, _) = engine
        .http
        .json(&descriptor.url, Some(&descriptor), &engine.cancel)?;
    let files = manifest["files"].as_object().ok_or(Error::Metadata)?;
    if files.len() > 10_000 {
        return Err(Error::Metadata);
    }
    links_valid(files)?;
    let key = platform_key(platform);
    let reference_path = format!("state/runtime-{key}-{component}.json");
    if let Ok(reference) = engine.store.read_json(&reference_path) {
        if reference["schemaVersion"] != 1 {
            return Err(Error::UnsupportedSchema);
        }
        if reference["manifestSha1"] == descriptor.sha1 {
            let generation = metadata::field(&reference, "generation")?;
            filesystem::relative(generation)?;
            if !generation.starts_with(&format!("shared/runtimes/{key}/{component}/")) {
                return Err(Error::UnsafePath);
            }
            let mut valid = true;
            let mut verified_bytes = 0u64;
            for (path, file) in files {
                engine.cancel.check()?;
                match metadata::field(file, "type")? {
                    "file" => {
                        let resource = Resource::new(
                            format!("{generation}/{path}"),
                            metadata::decode(&file["downloads"]["raw"])?,
                        )?;
                        valid &= engine.store.valid(&resource, &engine.cancel)?;
                        verified_bytes = verified_bytes
                            .checked_add(resource.download.size)
                            .ok_or(Error::Metadata)?;
                    }
                    "link" => {
                        valid &= engine.store.link_matches(
                            &format!("{generation}/{path}"),
                            metadata::field(file, "target")?,
                        )?;
                    }
                    "directory" => {
                        valid &= engine
                            .store
                            .directory_valid(&format!("{generation}/{path}"))?;
                    }
                    _ => return Err(Error::Metadata),
                }
            }
            let executable = match platform.os {
                OperatingSystem::Windows => "bin/java.exe",
                OperatingSystem::Linux => "bin/java",
                OperatingSystem::Macos => "jre.bundle/Contents/Home/bin/java",
            };
            if valid {
                let path = crate::runtime::execution_path(
                    &engine.store.path(&format!("{generation}/{executable}"))?,
                )?;
                match probe(&path, major, platform, &engine.cancel) {
                    Ok(()) => {
                        engine.add_reused(verified_bytes);
                        return Ok(path.to_str().ok_or(Error::UnsafePath)?.into());
                    }
                    Err(Error::Cancelled) => return Err(Error::Cancelled),
                    Err(_) => {}
                }
            }
        }
    }
    let staging = format!(
        "shared/runtimes/{key}/{component}/.staging-{}",
        uuid::Uuid::new_v4()
    );
    let generation = format!(
        "shared/runtimes/{key}/{component}/{}-{}",
        descriptor.sha1,
        uuid::Uuid::new_v4()
    );
    engine.store.directory(&staging)?;
    let mut resources = BTreeMap::new();
    for (path, file) in files {
        engine.cancel.check()?;
        match metadata::field(file, "type")? {
            "directory" => engine.store.directory(&format!("{staging}/{path}"))?,
            "file" => {
                let download: Download = metadata::decode(&file["downloads"]["raw"])?;
                let resource = Resource::new(
                    format!("shared/runtimes/objects/{}", download.sha1),
                    download,
                )?;
                engine.ensure(&resource)?;
                engine
                    .store
                    .copy(&resource.path, &format!("{staging}/{path}"))?;
                if file["executable"].as_bool().ok_or(Error::Metadata)? {
                    engine.store.executable(&format!("{staging}/{path}"))?;
                }
                resources.insert(path.clone(), resource.download);
            }
            "link" => {}
            _ => return Err(Error::Metadata),
        }
    }
    for (path, file) in files {
        if file["type"] == "link" {
            engine.store.link(
                &format!("{staging}/{path}"),
                metadata::field(file, "target")?,
            )?;
        }
    }
    let executable = match platform.os {
        OperatingSystem::Windows => "bin/java.exe",
        OperatingSystem::Linux => "bin/java",
        OperatingSystem::Macos => "jre.bundle/Contents/Home/bin/java",
    };
    if files
        .get(executable)
        .is_none_or(|v| v["type"] != "file" || v["executable"] != true)
    {
        return Err(Error::Metadata);
    }
    probe(
        &engine.store.path(&format!("{staging}/{executable}"))?,
        major,
        platform,
        &engine.cancel,
    )?;
    engine.cancel.check()?;
    engine.store.publish_directory(&staging, &generation)?;
    engine.store.document(&format!("state/runtime-{key}-{component}.json"),&serde_json::json!({"schemaVersion":1,"manifestSha1":descriptor.sha1,"generation":generation,"files":resources}))?;
    Ok(
        crate::runtime::execution_path(&engine.store.path(&format!("{generation}/{executable}"))?)?
            .to_str()
            .ok_or(Error::UnsafePath)?
            .into(),
    )
}
fn probe(
    path: &std::path::Path,
    major: u32,
    platform: Platform,
    cancel: &Cancellation,
) -> Result<()> {
    let path = crate::runtime::execution_path(path)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::RuntimeIncompatible)?;
    let bytes=runtime.block_on(async {
        use tokio::io::AsyncReadExt;
        let mut command=tokio::process::Command::new(&path);
        command.args(["-XshowSettings:properties","-version"]).kill_on_drop(true).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        for name in crate::runtime::JAVA_ENV_OVERRIDES {command.env_remove(name);}
        let mut child=command.spawn().map_err(|_|Error::RuntimeIncompatible)?;
        let stdout=child.stdout.take().ok_or(Error::RuntimeIncompatible)?;let stderr=child.stderr.take().ok_or(Error::RuntimeIncompatible)?;
        async fn read(reader:impl tokio::io::AsyncRead+Unpin)->std::io::Result<Vec<u8>> {
            let mut bytes=vec![];reader.take(65537).read_to_end(&mut bytes).await?;
            if bytes.len()>65536 {return Err(std::io::Error::other("probe output limit"));}Ok(bytes)
        }
        let result={
            let work=async {tokio::try_join!(child.wait(),read(stdout),read(stderr))};
            tokio::pin!(work);
            let timeout=tokio::time::sleep(Duration::from_secs(5));tokio::pin!(timeout);
            loop {tokio::select! {
                result=&mut work=>break result.map_err(|_|Error::RuntimeIncompatible),
                _=&mut timeout=>break Err(Error::RuntimeIncompatible),
                _=tokio::time::sleep(Duration::from_millis(20))=>{if cancel.check().is_err(){break Err(Error::Cancelled);}}
            }}
        };
        match result {
            Ok((status,mut stdout,stderr)) if status.success()=>{stdout.extend(stderr);Ok(stdout)},
            Ok(_)=>Err(Error::RuntimeIncompatible),
            Err(error)=>{let _=child.kill().await;let _=child.wait().await;Err(error)},
        }
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let property = |name: &str| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix(name)
                .and_then(|s| s.trim().strip_prefix('='))
                .map(str::trim)
        })
    };
    if property("java.specification.version") != Some(major.to_string().as_str())
        || property("os.arch")
            .map(crate::runtime::java_architecture)
            .transpose()?
            .is_none_or(|arch| arch != platform.architecture)
    {
        return Err(Error::RuntimeIncompatible);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_runtime_links_remain_inside_the_generation()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        for source in [
            include_str!("../../../../docs/research/runtime-mac-os-arm64.json"),
            include_str!("../../../../docs/research/runtime-mac-os.json"),
            include_str!("../../../../docs/research/runtime-linux.json"),
            include_str!("../../../../docs/research/runtime-windows-x64.json"),
        ] {
            let value: Value = serde_json::from_str(source)?;
            links_valid(value["files"].as_object().ok_or("files missing")?)?;
        }
        assert!(normalized_link("a/b", "../../outside").is_err());
        let cycle =
            serde_json::json!({"a":{"type":"link","target":"b"},"b":{"type":"link","target":"a"}});
        assert!(links_valid(cycle.as_object().ok_or("object missing")?).is_err());
        Ok(())
    }
}
