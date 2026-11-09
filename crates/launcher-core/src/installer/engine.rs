use super::{
    Cancellation, Http,
    metadata::{self, Download, Resource, RuleContext, VersionPlan},
};
use crate::{
    core::{
        EventSink, LauncherError as Error, OperationEvent, OperationId, OperationStage, Platform,
        Result,
    },
    profiles::{Profile, ProfileId},
    storage::ResourceStore,
};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

pub const VERSION_MANIFEST: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedInstallation {
    pub schema_version: u32,
    pub profile_id: ProfileId,
    pub metadata_sha1: String,
    pub asset_index_id: String,
    pub classpath: Vec<String>,
    pub java_executable: String,
    pub platform: Platform,
    pub downloaded_bytes: u64,
    pub reused_bytes: u64,
}

pub struct InstallEngine {
    pub(super) http: Http,
    pub(super) store: Arc<ResourceStore>,
    pub(super) cancel: Cancellation,
    operation: OperationId,
    profile: ProfileId,
    sequence: AtomicU64,
    downloaded: AtomicU64,
    reused: AtomicU64,
}
impl std::fmt::Debug for InstallEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstallEngine")
            .field("operation", &self.operation)
            .finish_non_exhaustive()
    }
}
impl InstallEngine {
    pub fn new(
        store: Arc<ResourceStore>,
        profile: ProfileId,
        cancel: Cancellation,
    ) -> Result<Self> {
        Ok(Self {
            http: Http::official()?,
            store,
            cancel,
            profile,
            operation: OperationId(uuid::Uuid::new_v4()),
            sequence: AtomicU64::new(0),
            downloaded: AtomicU64::new(0),
            reused: AtomicU64::new(0),
        })
    }
    pub(super) fn add_reused(&self, bytes: u64) {
        self.reused.fetch_add(bytes, Ordering::Relaxed);
    }
    pub fn cancellation(&self) -> Cancellation {
        self.cancel.clone()
    }
    pub(crate) fn resource_store(&self) -> Arc<ResourceStore> {
        Arc::clone(&self.store)
    }
    pub fn operation_id(&self) -> OperationId {
        self.operation
    }
    fn event(
        &self,
        stage: OperationStage,
        completed: u64,
        total: Option<u64>,
        events: &dyn EventSink,
    ) {
        events.publish(OperationEvent {
            operation_id: self.operation,
            profile_id: self.profile,
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
            stage,
            completed_bytes: completed,
            total_bytes: total,
        });
    }
    pub fn plan(&self, profile: &Profile, context: &RuleContext) -> Result<VersionPlan> {
        if profile.id != self.profile || profile.minecraft_version != "1.21.1" {
            return Err(Error::Metadata);
        }
        let (manifest, _) = self.http.json(VERSION_MANIFEST, None, &self.cancel)?;
        let descriptor = manifest["versions"]
            .as_array()
            .ok_or(Error::Metadata)?
            .iter()
            .find(|v| v["id"] == profile.minecraft_version)
            .ok_or(Error::Metadata)?;
        let url = metadata::field(descriptor, "url")?;
        let expected_hash = metadata::field(descriptor, "sha1")?;
        metadata::hash(expected_hash)?;
        let (v, bytes) = self.http.json(url, None, &self.cancel)?;
        use sha1::{Digest, Sha1};
        if format!("{:x}", Sha1::digest(&bytes)) != expected_hash {
            return Err(Error::Integrity);
        }
        let plan = metadata::version_plan(v, expected_hash, context)?;
        let resource = Resource::new(
            format!("shared/metadata/versions/{expected_hash}.json"),
            Download {
                url: url.into(),
                sha1: expected_hash.into(),
                size: bytes.len() as u64,
            },
        )?;
        self.store
            .publish(&resource, &bytes[..], &self.cancel, |_| {})?;
        Ok(plan)
    }
    pub(super) fn ensure(&self, resource: &Resource) -> Result<()> {
        resource.download.validate()?;
        self.http
            .validate(&reqwest::Url::parse(&resource.download.url).map_err(|_| Error::Metadata)?)?;
        if self.store.valid(resource, &self.cancel)? {
            self.reused
                .fetch_add(resource.download.size, Ordering::Relaxed);
            return Ok(());
        }
        for attempt in 0..3 {
            self.cancel.check()?;
            let result = self
                .http
                .response(&resource.download.url, &self.cancel)
                .and_then(|response| {
                    if response
                        .content_length()
                        .is_some_and(|size| size != resource.download.size)
                    {
                        return Err(Error::Integrity);
                    }
                    self.store.publish(resource, response, &self.cancel, |n| {
                        self.downloaded.fetch_add(n, Ordering::Relaxed);
                    })
                });
            self.cancel.check()?;
            match result {
                Ok(()) => return Ok(()),
                Err(Error::Network | Error::Integrity) if attempt < 2 => self
                    .cancel
                    .pause(Duration::from_millis(150 * (1 << attempt)))?,
                Err(e) => return Err(e),
            }
        }
        Err(Error::Network)
    }
    pub fn prepare(
        &self,
        plan: &VersionPlan,
        platform: Platform,
        events: &dyn EventSink,
        runtime: &crate::runtime::RuntimeSelection,
    ) -> Result<PreparedInstallation> {
        let outcome = self.prepare_inner(plan, platform, events, runtime);
        match &outcome {
            Ok(_) => self.event(OperationStage::Ready, 0, None, events),
            Err(Error::Cancelled) => self.event(OperationStage::Cancelled, 0, None, events),
            Err(_) => self.event(OperationStage::Error, 0, None, events),
        }
        outcome
    }
    fn prepare_inner(
        &self,
        plan: &VersionPlan,
        platform: Platform,
        events: &dyn EventSink,
        runtime: &crate::runtime::RuntimeSelection,
    ) -> Result<PreparedInstallation> {
        self.event(OperationStage::Initializing, 0, None, events);
        let index = Resource::new(
            format!(
                "shared/assets/indexes/{}-{}.json",
                plan.asset_index_id, plan.asset_index.sha1
            ),
            plan.asset_index.clone(),
        )?;
        self.ensure(&index)?;
        let mut resources = plan.resources.clone();
        resources.extend(metadata::assets(&self.store.read_json(&index.path)?)?);
        let total = resources.iter().try_fold(0u64, |acc, r| {
            acc.checked_add(r.download.size).ok_or(Error::Metadata)
        })?;
        self.event(OperationStage::Checking, 0, Some(total), events);
        let queue = Mutex::new(resources.iter());
        let failure = Mutex::new(None);
        let completed = AtomicU64::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    loop {
                        let next = match queue.lock() {
                            Ok(mut q) => q.next(),
                            Err(_) => {
                                return;
                            }
                        };
                        let Some(resource) = next else {
                            return;
                        };
                        if failure.lock().map_or(true, |f| f.is_some()) {
                            return;
                        }
                        match self.ensure(resource) {
                            Ok(()) => {
                                let done = completed
                                    .fetch_add(resource.download.size, Ordering::Relaxed)
                                    + resource.download.size;
                                self.event(OperationStage::Downloading, done, Some(total), events);
                            }
                            Err(e) => {
                                if let Ok(mut failure) = failure.lock()
                                    && failure.is_none()
                                {
                                    *failure = Some(e);
                                }
                                return;
                            }
                        }
                    }
                });
            }
        });
        if let Some(error) = failure.into_inner().map_err(|_| Error::StorageBusy)? {
            return Err(error);
        }
        self.cancel.check()?;
        self.event(OperationStage::Extracting, 0, None, events);
        let java_executable = super::java::install(
            self,
            &plan.java_component,
            plan.java_major,
            platform,
            runtime,
        )?;
        let result = PreparedInstallation {
            schema_version: 1,
            java_executable,
            profile_id: self.profile,
            metadata_sha1: plan.metadata_sha1.clone(),
            asset_index_id: format!("{}-{}", plan.asset_index_id, plan.asset_index.sha1),
            classpath: plan.classpath.clone(),
            platform,
            downloaded_bytes: self.downloaded.load(Ordering::Relaxed),
            reused_bytes: self.reused.load(Ordering::Relaxed),
        };
        self.store.document(
            &format!("state/installation-{}.json", self.profile.0),
            &result,
        )?;
        Ok(result)
    }
}
