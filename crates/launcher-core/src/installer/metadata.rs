//! Bounded interpretation of Mojang metadata, shared by install and future launch planning.
use crate::{
    core::{Architecture, LauncherError as Error, OperatingSystem, Platform, Result},
    storage::filesystem,
};
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Download {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}
impl Download {
    pub fn validate(&self) -> Result<()> {
        hash(&self.sha1)?;
        if self.size == 0 || self.size > 1024 * 1024 * 1024 {
            return Err(Error::Metadata);
        }
        Ok(())
    }
}
pub fn hash(value: &str) -> Result<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Metadata);
    }
    Ok(())
}
pub fn field<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key).and_then(Value::as_str).ok_or(Error::Metadata)
}
pub fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T> {
    serde_json::from_value(v.clone()).map_err(|_| Error::Metadata)
}

#[derive(Debug, Clone)]
pub struct RuleContext {
    pub platform: Platform,
    pub os_version: String,
    pub features: BTreeMap<String, bool>,
}
impl RuleContext {
    pub fn new(platform: Platform, os_version: String) -> Self {
        Self {
            platform,
            os_version,
            features: BTreeMap::new(),
        }
    }
}
pub fn allowed(rules: Option<&Value>, context: &RuleContext) -> Result<bool> {
    let Some(rules) = rules else {
        return Ok(true);
    };
    let rules = rules.as_array().ok_or(Error::Metadata)?;
    let mut allow = false;
    for rule in rules {
        let object = rule.as_object().ok_or(Error::Metadata)?;
        if object
            .keys()
            .any(|k| !["action", "os", "features"].contains(&k.as_str()))
        {
            return Err(Error::Metadata);
        }
        let action = match field(rule, "action")? {
            "allow" => true,
            "disallow" => false,
            _ => return Err(Error::Metadata),
        };
        let mut matches = true;
        if let Some(os) = rule.get("os") {
            for (key, value) in os.as_object().ok_or(Error::Metadata)? {
                let expected = value.as_str().ok_or(Error::Metadata)?;
                let actual = match key.as_str() {
                    "name" => match context.platform.os {
                        OperatingSystem::Windows => "windows",
                        OperatingSystem::Linux => "linux",
                        OperatingSystem::Macos => "osx",
                    },
                    "arch" => match context.platform.architecture {
                        Architecture::X86_64 => "amd64",
                        Architecture::Aarch64 => "aarch64",
                    },
                    "version" => &context.os_version,
                    _ => return Err(Error::Metadata),
                };
                if key == "name" {
                    matches &= actual == expected;
                } else {
                    if expected.len() > 1024 {
                        return Err(Error::Metadata);
                    }
                    matches &= RegexBuilder::new(expected)
                        .size_limit(1024 * 1024)
                        .dfa_size_limit(1024 * 1024)
                        .build()
                        .map_err(|_| Error::Metadata)?
                        .is_match(actual);
                }
            }
        }
        if let Some(features) = rule.get("features") {
            for (key, value) in features.as_object().ok_or(Error::Metadata)? {
                matches &= context.features.get(key).copied().unwrap_or(false)
                    == value.as_bool().ok_or(Error::Metadata)?;
            }
        }
        if matches {
            allow = action;
        }
    }
    Ok(allow)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resource {
    pub path: String,
    pub download: Download,
}
impl Resource {
    pub fn new(path: String, download: Download) -> Result<Self> {
        filesystem::relative(&path)?;
        download.validate()?;
        if !path.starts_with("shared/") {
            return Err(Error::UnsafePath);
        }
        Ok(Self { path, download })
    }
}
#[derive(Debug, Clone)]
pub struct VersionPlan {
    pub metadata_sha1: String,
    pub main_class: String,
    pub java_component: String,
    pub java_major: u32,
    pub asset_index_id: String,
    pub asset_index: Download,
    pub resources: Vec<Resource>,
    pub classpath: Vec<String>,
    pub metadata: Value,
}
pub fn version_plan(v: Value, metadata_sha1: &str, context: &RuleContext) -> Result<VersionPlan> {
    hash(metadata_sha1)?;
    if field(&v, "id")? != "1.21.1" || field(&v, "type")? != "release" || v.get("natives").is_some()
    {
        return Err(Error::Metadata);
    }
    let main_class = field(&v, "mainClass")?.to_owned();
    if main_class != "net.minecraft.client.main.Main" {
        return Err(Error::Metadata);
    }
    let java = v.get("javaVersion").ok_or(Error::Metadata)?;
    let java_component = field(java, "component")?.to_owned();
    filesystem::relative(&java_component)?;
    if java_component.contains('/') {
        return Err(Error::Metadata);
    }
    let java_major = java
        .get("majorVersion")
        .and_then(Value::as_u64)
        .ok_or(Error::Metadata)?;
    if java_major != 21 {
        return Err(Error::Metadata);
    }
    let arguments = v.get("arguments").ok_or(Error::Metadata)?;
    for key in ["game", "jvm"] {
        argument_values(arguments.get(key).ok_or(Error::Metadata)?, context)?;
    }
    let mut resources = vec![];
    let mut classpath = vec![];
    for lib in v
        .get("libraries")
        .and_then(Value::as_array)
        .ok_or(Error::Metadata)?
    {
        if lib.get("natives").is_some() || lib["downloads"].get("classifiers").is_some() {
            return Err(Error::Metadata);
        }
        if !allowed(lib.get("rules"), context)? {
            continue;
        }
        let artifact = &lib["downloads"]["artifact"];
        let path = field(artifact, "path")?;
        filesystem::relative(path)?;
        // Hash revision prevents replacing a library currently used by another profile.
        let download: Download = decode(artifact)?;
        let path = format!("shared/libraries/{}/{path}", download.sha1);
        resources.push(Resource::new(path.clone(), download)?);
        classpath.push(path);
    }
    if resources.is_empty() || resources.len() > 512 {
        return Err(Error::Metadata);
    }
    let client: Download = decode(&v["downloads"]["client"])?;
    let client_path = format!("shared/versions/1.21.1/{metadata_sha1}/client.jar");
    resources.push(Resource::new(client_path.clone(), client)?);
    classpath.push(client_path);
    let index = &v["assetIndex"];
    let asset_index_id = field(index, "id")?.to_owned();
    filesystem::relative(&asset_index_id)?;
    if asset_index_id.contains('/') {
        return Err(Error::Metadata);
    }
    let asset_index: Download = decode(index)?;
    asset_index.validate()?;
    let log = &v["logging"]["client"];
    if field(log, "type")? != "log4j2-xml"
        || field(log, "argument")? != "-Dlog4j.configurationFile=${path}"
    {
        return Err(Error::Metadata);
    }
    let logging: Download = decode(&log["file"])?;
    resources.push(Resource::new(
        format!("shared/metadata/logging/{}.xml", logging.sha1),
        logging,
    )?);
    let mut seen = BTreeSet::new();
    for r in &resources {
        if !seen.insert(r.path.clone()) {
            return Err(Error::Metadata);
        }
    }
    Ok(VersionPlan {
        metadata_sha1: metadata_sha1.into(),
        main_class,
        java_component,
        java_major: java_major as u32,
        asset_index_id,
        asset_index,
        resources,
        classpath,
        metadata: v,
    })
}
pub fn argument_values(v: &Value, context: &RuleContext) -> Result<Vec<String>> {
    let mut result: Vec<String> = vec![];
    for value in v.as_array().ok_or(Error::Metadata)? {
        if let Some(s) = value.as_str() {
            result.push(s.into());
            continue;
        }
        let object = value.as_object().ok_or(Error::Metadata)?;
        if object
            .keys()
            .any(|k| !["rules", "value"].contains(&k.as_str()))
        {
            return Err(Error::Metadata);
        }
        if !allowed(value.get("rules"), context)? {
            continue;
        }
        let argument = value.get("value").ok_or(Error::Metadata)?;
        if let Some(s) = argument.as_str() {
            result.push(s.into());
        } else {
            for s in argument.as_array().ok_or(Error::Metadata)? {
                result.push(s.as_str().ok_or(Error::Metadata)?.into());
            }
        }
    }
    if result.len() > 256 || result.iter().any(|s| s.len() > 4096 || s.contains('\0')) {
        return Err(Error::Metadata);
    }
    Ok(result)
}
pub fn assets(v: &Value) -> Result<Vec<Resource>> {
    if v.get("virtual").and_then(Value::as_bool).unwrap_or(false)
        || v.get("map_to_resources")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Err(Error::Metadata);
    }
    let mut unique = BTreeMap::new();
    let objects = v["objects"].as_object().ok_or(Error::Metadata)?;
    if objects.len() > 100_000 {
        return Err(Error::Metadata);
    }
    for object in objects.values() {
        let h = field(object, "hash")?;
        hash(h)?;
        let size = object["size"].as_u64().ok_or(Error::Metadata)?;
        if let Some(old) = unique.insert(h.to_owned(), size)
            && old != size
        {
            return Err(Error::Metadata);
        }
    }
    unique
        .into_iter()
        .map(|(h, size)| {
            Resource::new(
                format!("shared/assets/objects/{}/{h}", &h[..2]),
                Download {
                    url: format!("https://resources.download.minecraft.net/{}/{h}", &h[..2]),
                    sha1: h,
                    size,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_fixtures_build_all_four_native_plans()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let value: Value =
            serde_json::from_str(include_str!("../../../../docs/research/1.21.1.json"))?;
        for (os, architecture) in [
            (OperatingSystem::Windows, Architecture::X86_64),
            (OperatingSystem::Linux, Architecture::X86_64),
            (OperatingSystem::Macos, Architecture::X86_64),
            (OperatingSystem::Macos, Architecture::Aarch64),
        ] {
            let context = RuleContext::new(Platform { os, architecture }, "10.0".into());
            let plan = version_plan(
                value.clone(),
                "82ba6501da174a665c1787775f946487422f2432",
                &context,
            )?;
            assert!(!plan.classpath.is_empty());
            assert_eq!(plan.java_major, 21);
            let args = argument_values(&value["arguments"]["jvm"], &context)?;
            assert_eq!(
                args.iter().any(|s| s == "-XstartOnFirstThread"),
                os == OperatingSystem::Macos
            );
            assert!(
                !argument_values(&value["arguments"]["game"], &context)?
                    .iter()
                    .any(|s| s == "--demo")
            );
        }
        let index: Value = serde_json::from_str(include_str!(
            "../../../../docs/research/asset-index-17.json"
        ))?;
        assert_eq!(assets(&index)?.len(), 3888);
        Ok(())
    }
    #[test]
    fn rules_are_ordered_bounded_and_fail_closed() -> Result<()> {
        let context = RuleContext::new(
            Platform {
                os: OperatingSystem::Linux,
                architecture: Architecture::X86_64,
            },
            "6.8".into(),
        );
        assert!(allowed(None, &context)?);
        assert!(!allowed(Some(&serde_json::json!([])), &context)?);
        assert!(!allowed(
            Some(
                &serde_json::json!([{"action":"allow"},{"action":"disallow","os":{"name":"linux","arch":"amd64","version":"^6\\."}}])
            ),
            &context
        )?);
        assert!(allowed(
            Some(&serde_json::json!([{"action":"allow","features":{"is_demo_user":false}}])),
            &context
        )?);
        assert!(
            allowed(
                Some(&serde_json::json!([{"action":"allow","unknown":true}])),
                &context
            )
            .is_err()
        );
        assert!(
            allowed(
                Some(&serde_json::json!([{"action":"allow","os":{"arch":"["}}])),
                &context
            )
            .is_err()
        );
        Ok(())
    }
}
