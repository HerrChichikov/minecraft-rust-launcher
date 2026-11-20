//! Only opaque platform refresh tokens are retained in the native OS store.
use super::{SecretString, platform::valid_token};
use crate::core::{LauncherError as Error, Result};
use serde::{Deserialize, Serialize};

pub const REFERENCE: &str = "platform/current";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Record {
    pub schema_version: u32,
    pub origin: String,
    pub refresh: SecretString,
}

pub(super) fn encode(origin: &str, refresh: &SecretString) -> Result<SecretString> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire<'a> {
        schema_version: u32,
        origin: &'a str,
        refresh: &'a str,
    }
    serde_json::to_string(&Wire {
        schema_version: 1,
        origin,
        refresh: refresh.expose_secret(),
    })
    .map(SecretString::new)
    .map_err(|_| Error::AuthProtocol)
}

pub(super) fn decode(secret: &SecretString, origin: &str) -> Result<Record> {
    if secret.expose_secret().len() > 4096 {
        return Err(Error::AuthProtocol);
    }
    let r: Record =
        serde_json::from_str(secret.expose_secret()).map_err(|_| Error::AuthProtocol)?;
    if r.schema_version != 1 {
        return Err(Error::UnsupportedSchema);
    }
    if r.origin != origin || !valid_token(&r.refresh, "ur_") {
        return Err(Error::AuthProtocol);
    }
    Ok(r)
}
