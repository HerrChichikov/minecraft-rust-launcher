use super::*;
use crate::core::LauncherError as Error;
use serde::{Deserialize, Serialize};
use std::{io::Read, sync::OnceLock, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct ElyByConfiguration {
    pub provider_id: &'static str,
    pub authentication_base: &'static str,
    pub injector_api_root: &'static str,
}
impl Default for ElyByConfiguration {
    fn default() -> Self {
        Self {
            provider_id: "ely-by",
            authentication_base: "https://authserver.ely.by",
            injector_api_root: "https://account.ely.by/api/authlib-injector",
        }
    }
}
#[derive(Debug, Default)]
pub struct ElyByAuthProvider {
    client: OnceLock<Option<reqwest::blocking::Client>>,
    #[cfg(test)]
    test_base: Option<String>,
}
#[derive(Deserialize)]
struct ProfileWire {
    id: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionWire {
    access_token: SecretString,
    client_token: SecretString,
    selected_profile: ProfileWire,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorWire {
    error: String,
    error_message: String,
}
struct SecretBody {
    bytes: Zeroizing<Vec<u8>>,
    offset: usize,
}
impl std::fmt::Debug for SecretBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBody([REDACTED])")
    }
}
impl Read for SecretBody {
    fn read(&mut self, dest: &mut [u8]) -> std::io::Result<usize> {
        let n = dest.len().min(self.bytes.len() - self.offset);
        dest[..n].copy_from_slice(&self.bytes[self.offset..self.offset + n]);
        self.offset += n;
        Ok(n)
    }
}
impl ElyByAuthProvider {
    fn post(&self, action: &str, payload: &impl Serialize) -> Result<(u16, Zeroizing<Vec<u8>>)> {
        let client = self
            .client
            .get_or_init(|| {
                reqwest::blocking::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(5))
                    .timeout(Duration::from_secs(15))
                    .user_agent("Launcher/0.1.0")
                    .build()
                    .ok()
            })
            .as_ref()
            .ok_or(Error::AuthNetwork)?;
        let base = ElyByConfiguration::default().authentication_base;
        #[cfg(test)]
        let base = self.test_base.as_deref().unwrap_or(base);
        let bytes = Zeroizing::new(serde_json::to_vec(payload).map_err(|_| Error::AuthProtocol)?);
        let length = bytes.len();
        let response = client
            .post(format!("{base}/auth/{action}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::CONTENT_LENGTH, length)
            .body(reqwest::blocking::Body::new(SecretBody {
                bytes,
                offset: 0,
            }))
            .send()
            .map_err(|_| Error::AuthNetwork)?;
        let status = response.status().as_u16();
        if response
            .headers()
            .contains_key(reqwest::header::CONTENT_ENCODING)
            || response.content_length().is_some_and(|n| n > 128 * 1024)
        {
            return Err(Error::AuthProtocol);
        }
        let mut bytes = Zeroizing::new(vec![]);
        response
            .take(128 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::AuthNetwork)?;
        if bytes.len() > 128 * 1024 {
            return Err(Error::AuthProtocol);
        }
        if status == 429 || status >= 500 {
            return Err(Error::AuthNetwork);
        }
        // Never resend auth bodies to a redirect destination or retry token rotation automatically.
        if (300..400).contains(&status) {
            return Err(Error::AuthProtocol);
        }
        Ok((status, bytes))
    }
    fn session(&self, body: &[u8], client: &SecretString) -> Result<Session> {
        let wire: SessionWire = serde_json::from_slice(body).map_err(|_| Error::AuthProtocol)?;
        let id = wire
            .selected_profile
            .id
            .parse::<Uuid>()
            .map_err(|_| Error::AuthProtocol)?;
        let name = wire.selected_profile.name;
        if id.is_nil()
            || !(3..=16).contains(&name.len())
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || wire.client_token.expose_secret() != client.expose_secret()
            || wire.access_token.expose_secret().is_empty()
            || wire.access_token.expose_secret().len() > 16384
            || !wire
                .access_token
                .expose_secret()
                .bytes()
                .all(|b| (33..=126).contains(&b))
        {
            return Err(Error::AuthProtocol);
        }
        Ok(Session {
            provider_id: self.id().into(),
            profile: PlayerProfile { id, name },
            access_token: wire.access_token,
            client_token: Some(wire.client_token),
        })
    }
    pub(crate) fn password(
        &self,
        login: &str,
        password: &SecretString,
        totp: Option<&SecretString>,
        client: &SecretString,
    ) -> Result<AuthOutcome> {
        if login.is_empty()
            || login.len() > 254
            || login.chars().any(char::is_control)
            || password.expose_secret().is_empty()
            || password.expose_secret().len() > 4096
            || totp.is_some_and(|t| {
                t.expose_secret().len() != 6
                    || !t.expose_secret().bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err(Error::InvalidInput {
                field: "credentials",
            });
        }
        let combined = Zeroizing::new(match totp {
            Some(t) => format!("{}:{}", password.expose_secret(), t.expose_secret()),
            None => password.expose_secret().into(),
        });
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            username: &'a str,
            password: &'a str,
            client_token: &'a str,
            request_user: bool,
        }
        let (status, body) = self.post(
            "authenticate",
            &Request {
                username: login,
                password: &combined,
                client_token: client.expose_secret(),
                request_user: false,
            },
        )?;
        if status == 200 {
            return self.session(&body, client).map(AuthOutcome::Authenticated);
        }
        if status == 401 || status == 403 {
            if status == 401
                && serde_json::from_slice::<ErrorWire>(&body).is_ok_and(|e| {
                    e.error == "ForbiddenOperationException"
                        && e.error_message == "Account protected with two factor auth."
                })
            {
                return Ok(AuthOutcome::TotpRequired {
                    challenge_id: Uuid::new_v4(),
                });
            }
            return Err(Error::AuthRejected);
        }
        Err(Error::AuthProtocol)
    }
    fn client<'a>(&self, session: &'a Session) -> Result<&'a SecretString> {
        if session.provider_id != self.id() {
            return Err(Error::AuthUnavailable);
        }
        session
            .client_token
            .as_ref()
            .ok_or(Error::AuthenticationRequired)
    }
}
impl AuthProvider for ElyByAuthProvider {
    fn id(&self) -> &str {
        "ely-by"
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            available: true,
            development: false,
            flows: vec!["password_totp".into()],
            unavailable_reason: None,
        }
    }
    fn authenticate(&self, input: AuthInput) -> Result<AuthOutcome> {
        match input {
            AuthInput::Password {
                login,
                password,
                totp,
            } => self.password(
                &login,
                &password,
                totp.as_ref(),
                &SecretString::new(Uuid::new_v4().to_string()),
            ),
            #[cfg(all(debug_assertions, feature = "dev-auth"))]
            _ => Err(Error::UnsupportedAuthFlow),
        }
    }
    fn refresh(&self, session: &Session) -> Result<Session> {
        let client = self.client(session)?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            access_token: &'a str,
            client_token: &'a str,
            request_user: bool,
        }
        let (status, body) = self.post(
            "refresh",
            &Request {
                access_token: session.access_token.expose_secret(),
                client_token: client.expose_secret(),
                request_user: false,
            },
        )?;
        if status == 400 || status == 401 || status == 403 {
            return Err(Error::AuthenticationRequired);
        }
        if status != 200 {
            return Err(Error::AuthProtocol);
        }
        let fresh = self.session(&body, client)?;
        if fresh.profile.id != session.profile.id {
            return Err(Error::AuthProtocol);
        }
        Ok(fresh)
    }
    fn validate(&self, session: &Session) -> Result<SessionValidity> {
        self.client(session)?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            access_token: &'a str,
        }
        let (status, _) = self.post(
            "validate",
            &Request {
                access_token: session.access_token.expose_secret(),
            },
        )?;
        match status {
            200 | 204 => Ok(SessionValidity::Valid),
            400 | 401 | 403 => Ok(SessionValidity::ReauthenticationRequired),
            _ => Err(Error::AuthProtocol),
        }
    }
    fn logout(&self, session: &Session) -> Result<()> {
        let client = self.client(session)?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            access_token: &'a str,
            client_token: &'a str,
        }
        let (status, _) = self.post(
            "invalidate",
            &Request {
                access_token: session.access_token.expose_secret(),
                client_token: client.expose_secret(),
            },
        )?;
        if status == 200 || status == 204 {
            Ok(())
        } else {
            Err(Error::AuthProtocol)
        }
    }
    fn launch_identity(&self, session: &Session) -> Result<LaunchIdentity> {
        self.client(session)?;
        Ok(LaunchIdentity {
            profile: session.profile.clone(),
            access_token: SecretString::new(session.access_token.expose_secret().into()),
            user_type: "mojang".into(),
            requires_ely_by_agent: true,
        })
    }
}
#[cfg(test)]
mod tests;
