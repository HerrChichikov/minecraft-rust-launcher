//! Pinned official agent download and fresh SHA256 verification before producing JVM arguments.
use crate::{
    core::{LauncherError as Error, Result},
    installer::{
        Cancellation,
        metadata::{Download, Resource},
    },
    storage::ResourceStore,
};
use sha2::{Digest, Sha256};
use std::{ffi::OsString, io::Read, sync::Arc, time::Duration};

pub const BUILD: u32 = 56;
pub const VERSION: &str = "1.2.8";
pub const SHA256: &str = "9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10";
pub const DOWNLOAD_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar";
pub const SOURCE_COMMIT: &str = "5437f5b465ee3f5be0a1e5e1ae3c50978114f6fb";
const LIMIT: u64 = 8 * 1024 * 1024;
const PATH: &str = "shared/agents/authlib-injector/9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10/agent.jar";

fn verified(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() as u64 <= LIMIT
        && Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            == SHA256
}

#[derive(Debug)]
pub struct VerifiedAgent {
    store: Arc<ResourceStore>,
}
impl VerifiedAgent {
    /// Call again immediately before spawn; cached descriptors are never hash authority.
    pub fn jvm_arguments(&self, cancel: &Cancellation) -> Result<Vec<OsString>> {
        cancel.check()?;
        let bytes = self
            .store
            .bounded_bytes(PATH, LIMIT)?
            .ok_or(Error::Integrity)?;
        if !verified(&bytes) {
            return Err(Error::Integrity);
        }
        cancel.check()?;
        let path = crate::runtime::execution_path(&self.store.path(PATH)?)?;
        let text = path.to_str().ok_or(Error::UnsafePath)?;
        if text.contains('=') {
            return Err(Error::UnsafePath);
        }
        Ok(vec![
            "-Dauthlibinjector.noLogFile".into(),
            format!(
                "-javaagent:{text}={}",
                super::ElyByConfiguration::default().injector_api_root
            )
            .into(),
        ])
    }
}

pub fn ensure(store: Arc<ResourceStore>, cancel: &Cancellation) -> Result<VerifiedAgent> {
    cancel.check()?;
    if store
        .bounded_bytes(PATH, LIMIT)?
        .as_deref()
        .is_some_and(verified)
    {
        return Ok(VerifiedAgent { store });
    }
    // No redirects: the reviewed URL is the only executable network authority.
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("Launcher/0.1.0")
        .build()
        .map_err(|_| Error::Network)?;
    for attempt in 0..3 {
        cancel.check()?;
        let outcome = download(&client, cancel).and_then(|bytes| publish(&store, &bytes, cancel));
        match outcome {
            Ok(()) => return Ok(VerifiedAgent { store }),
            Err(Error::Network | Error::Integrity) if attempt < 2 => {
                cancel.pause(Duration::from_millis(150 * (1 << attempt)))?;
            }
            Err(error) => return Err(error),
        }
    }
    Err(Error::Network)
}
fn download(client: &reqwest::blocking::Client, cancel: &Cancellation) -> Result<Vec<u8>> {
    let mut response = client
        .get(DOWNLOAD_URL)
        .send()
        .map_err(|_| Error::Network)?;
    if !response.status().is_success()
        || response
            .headers()
            .contains_key(reqwest::header::CONTENT_ENCODING)
    {
        return Err(Error::Network);
    }
    if response.content_length().is_some_and(|n| n > LIMIT) {
        return Err(Error::Integrity);
    }
    let mut bytes = vec![];
    let mut buffer = [0u8; 65536];
    loop {
        cancel.check()?;
        let n = response.read(&mut buffer).map_err(|_| Error::Network)?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > LIMIT {
            return Err(Error::Integrity);
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
    if !verified(&bytes) {
        return Err(Error::Integrity);
    }
    Ok(bytes)
}
fn publish(store: &ResourceStore, bytes: &[u8], cancel: &Cancellation) -> Result<()> {
    if !verified(bytes) {
        return Err(Error::Integrity);
    }
    // The adapter's SHA1 adds an atomic streaming-write check after pinned SHA256 verification.
    use sha1::{Digest as Sha1Digest, Sha1};
    let resource = Resource::new(
        PATH.into(),
        Download {
            url: DOWNLOAD_URL.into(),
            sha1: format!("{:x}", Sha1::digest(bytes)),
            size: bytes.len() as u64,
        },
    )?;
    store.publish(&resource, bytes, cancel, |_| {})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_agent_and_oversized_bodies_fail_closed() {
        assert!(!verified(b"untrusted executable"));
        assert!(!verified(&[]));
        assert!(!verified(&vec![0; LIMIT as usize + 1]));
        assert!(DOWNLOAD_URL.starts_with("https://authlib-injector.yushi.moe/artifact/56/"));
    }
}
