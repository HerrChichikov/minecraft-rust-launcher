use crate::core::{LauncherError as Error, Result};
use reqwest::{
    Url,
    blocking::{Client, Response},
    redirect::Policy,
};
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Debug, Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub(crate) fn pause(&self, duration: Duration) -> Result<()> {
        let end = std::time::Instant::now() + duration;
        while std::time::Instant::now() < end {
            self.check()?;
            std::thread::sleep(Duration::from_millis(20));
        }
        self.check()
    }
}
#[derive(Debug, Clone)]
pub struct Http {
    client: Client,
    #[cfg(test)]
    loopback: Option<String>,
}
fn official(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && matches!(
            url.host_str(),
            Some(
                "piston-meta.mojang.com"
                    | "piston-data.mojang.com"
                    | "launchermeta.mojang.com"
                    | "launcher.mojang.com"
                    | "libraries.minecraft.net"
                    | "resources.download.minecraft.net"
            )
        )
}
impl Http {
    pub fn official() -> Result<Self> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("Launcher/0.1.0")
            .build()
            .map_err(|_| Error::Network)?;
        Ok(Self {
            client,
            #[cfg(test)]
            loopback: None,
        })
    }
    pub(crate) fn validate(&self, url: &Url) -> Result<()> {
        #[cfg(test)]
        if self
            .loopback
            .as_deref()
            .is_some_and(|base| url.as_str().starts_with(base))
        {
            return Ok(());
        }
        if !official(url) {
            return Err(Error::Metadata);
        }
        Ok(())
    }
    pub(crate) fn response(&self, source: &str, cancel: &Cancellation) -> Result<Response> {
        let mut url = Url::parse(source).map_err(|_| Error::Metadata)?;
        for _ in 0..=5 {
            cancel.check()?;
            self.validate(&url)?;
            let response = self
                .client
                .get(url.clone())
                .send()
                .map_err(|_| Error::Network)?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|h| h.to_str().ok())
                    .ok_or(Error::Network)?;
                url = url.join(location).map_err(|_| Error::Metadata)?;
                continue;
            }
            if !response.status().is_success() {
                return Err(
                    if response.status().is_server_error() || response.status().as_u16() == 429 {
                        Error::Network
                    } else {
                        Error::Metadata
                    },
                );
            }
            if response
                .headers()
                .contains_key(reqwest::header::CONTENT_ENCODING)
            {
                return Err(Error::Metadata);
            }
            return Ok(response);
        }
        Err(Error::Network)
    }
    pub(crate) fn json(
        &self,
        url: &str,
        expected: Option<&super::metadata::Download>,
        cancel: &Cancellation,
    ) -> Result<(serde_json::Value, Vec<u8>)> {
        for attempt in 0..3 {
            match self.json_once(url, expected, cancel) {
                Ok(value) => return Ok(value),
                Err(Error::Network | Error::Integrity) if attempt < 2 => {
                    cancel.pause(Duration::from_millis(150 * (1 << attempt)))?
                }
                Err(error) => return Err(error),
            }
        }
        Err(Error::Network)
    }
    fn json_once(
        &self,
        url: &str,
        expected: Option<&super::metadata::Download>,
        cancel: &Cancellation,
    ) -> Result<(serde_json::Value, Vec<u8>)> {
        let mut response = self.response(url, cancel)?;
        let limit = expected.map_or(16 * 1024 * 1024, |d| d.size.min(16 * 1024 * 1024));
        let mut bytes = vec![];
        let mut buffer = [0u8; 65536];
        loop {
            cancel.check()?;
            let n = response.read(&mut buffer).map_err(|_| Error::Network)?;
            if n == 0 {
                break;
            }
            if bytes.len() as u64 + n as u64 > limit {
                return Err(Error::Integrity);
            }
            bytes.extend_from_slice(&buffer[..n]);
        }
        if let Some(expected) = expected {
            use sha1::{Digest, Sha1};
            if bytes.len() as u64 != expected.size
                || format!("{:x}", Sha1::digest(&bytes)) != expected.sha1
            {
                return Err(Error::Integrity);
            }
        }
        let value = serde_json::from_slice(&bytes).map_err(|_| Error::Metadata)?;
        Ok((value, bytes))
    }
}

#[cfg(test)]
impl Http {
    pub(super) fn test(base: &str) -> Result<Self> {
        let mut http = Self::official()?;
        http.loopback = Some(base.into());
        Ok(http)
    }
}
