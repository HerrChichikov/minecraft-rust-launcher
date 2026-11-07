//! Private OS-store record, deliberately separate from all public IPC DTOs.
use super::*;
use crate::core::LauncherError as Error;
use serde::{Deserialize, Serialize};
pub(super) const REFERENCE: &str = "ely-by/current";
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    schema_version: u32,
    provider_id: String,
    profile_id: Uuid,
    profile_name: String,
    access_token: SecretString,
    client_token: SecretString,
}
pub(super) fn encode(session: &Session) -> Result<SecretString> {
    if session.provider_id != "ely-by" {
        return Err(Error::AuthUnavailable);
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Record<'a> {
        schema_version: u32,
        provider_id: &'a str,
        profile_id: Uuid,
        profile_name: &'a str,
        access_token: &'a str,
        client_token: &'a str,
    }
    let record = Record {
        schema_version: 1,
        provider_id: "ely-by",
        profile_id: session.profile.id,
        profile_name: &session.profile.name,
        access_token: session.access_token.expose_secret(),
        client_token: session
            .client_token
            .as_ref()
            .ok_or(Error::AuthenticationRequired)?
            .expose_secret(),
    };
    serde_json::to_string(&record)
        .map(SecretString::new)
        .map_err(|_| Error::AuthProtocol)
}
pub(super) fn decode(secret: &SecretString) -> Result<Session> {
    if secret.expose_secret().len() > 64 * 1024 {
        return Err(Error::AuthProtocol);
    }
    let r: Record =
        serde_json::from_str(secret.expose_secret()).map_err(|_| Error::AuthProtocol)?;
    if r.schema_version != 1 {
        return Err(Error::UnsupportedSchema);
    }
    if r.provider_id != "ely-by"
        || r.profile_id.is_nil()
        || !(3..=16).contains(&r.profile_name.len())
        || !r
            .profile_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || r.access_token.expose_secret().is_empty()
        || r.access_token.expose_secret().len() > 16384
        || !r
            .access_token
            .expose_secret()
            .bytes()
            .all(|b| (33..=126).contains(&b))
        || r.client_token.expose_secret().parse::<Uuid>().is_err()
    {
        return Err(Error::AuthProtocol);
    }
    Ok(Session {
        provider_id: r.provider_id,
        profile: PlayerProfile {
            id: r.profile_id,
            name: r.profile_name,
        },
        access_token: r.access_token,
        client_token: Some(r.client_token),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_record_cannot_restore_mock_or_malformed_future_sessions() {
        for provider in ["development-mock", "unknown"] {
            let r = format!(
                "{{\"schemaVersion\":1,\"providerId\":\"{provider}\",\"profileId\":\"12345678-1234-4234-8123-123456789abc\",\"profileName\":\"Notch\",\"accessToken\":\"fixture-token\",\"clientToken\":\"12345678-1234-4234-8123-123456789abc\"}}"
            );
            assert!(decode(&SecretString::new(r)).is_err());
        }
        assert!(decode(&SecretString::new("not-json".into())).is_err());
        let r=SecretString::new("{\"schemaVersion\":99,\"providerId\":\"ely-by\",\"profileId\":\"12345678-1234-4234-8123-123456789abc\",\"profileName\":\"Notch\",\"accessToken\":\"fixture-token\",\"clientToken\":\"12345678-1234-4234-8123-123456789abc\"}".into());
        assert!(matches!(decode(&r), Err(Error::UnsupportedSchema)));
    }
}
