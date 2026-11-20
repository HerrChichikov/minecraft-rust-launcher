use super::{PlayerProfile, SecretString};
use crate::core::{LauncherError as Error, Result};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, ClientBuilder, RequestBuilder, Response},
    redirect::Policy,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{io::Read, time::Duration};
use uuid::Uuid;
use zeroize::Zeroizing;

const LIMIT: u64 = 64 * 1024;
pub const PROVIDER_ID: &str = "platform";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Account {
    pub id: Uuid,
    pub nickname: String,
}
impl Account {
    pub(super) fn validate(&self) -> Result<()> {
        if self.id.is_nil() || !(1..=50).contains(&self.nickname.chars().count()) {
            return Err(Error::AuthProtocol);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameProfile {
    pub id: Uuid,
    pub realm: String,
    pub nickname: String,
}
impl GameProfile {
    fn validate(&self) -> Result<()> {
        if self.id.is_nil()
            || !(3..=16).contains(&self.nickname.len())
            || !self
                .nickname
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || !(1..=64).contains(&self.realm.len())
            || !self
                .realm
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(Error::AuthProtocol);
        }
        Ok(())
    }
    pub(super) fn player(&self) -> PlayerProfile {
        PlayerProfile {
            id: self.id,
            name: self.nickname.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TokenPair {
    pub access_token: SecretString,
    pub refresh_token: SecretString,
    pub session_id: Uuid,
}
impl TokenPair {
    fn validate(&self) -> Result<()> {
        if self.session_id.is_nil()
            || !valid_token(&self.access_token, "ua_")
            || !valid_token(&self.refresh_token, "ur_")
        {
            return Err(Error::AuthProtocol);
        }
        Ok(())
    }
}

pub(super) fn valid_token(token: &SecretString, prefix: &str) -> bool {
    token
        .expose_secret()
        .strip_prefix(prefix)
        .is_some_and(|value| {
            value.len() == 43
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

#[derive(Debug, Deserialize)]
pub(super) struct LoginResult {
    #[serde(flatten)]
    pub tokens: TokenPair,
    pub user: Account,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GameToken {
    pub access_token: SecretString,
    pub profile: GameProfile,
    injector_path: String,
}

#[derive(Debug)]
pub struct PlatformClient {
    base: Url,
    http: Client,
}

impl PlatformClient {
    pub fn new(base: &str) -> Result<Self> {
        Self::from_builder(parse_origin(base)?, Client::builder())
    }

    /// Use the isolated smoke fixture's trust root on an HTTPS loopback connection.
    #[cfg(all(debug_assertions, feature = "dev-auth"))]
    pub fn with_test_ca(base: &str, pem: &[u8]) -> Result<Self> {
        let base = parse_origin(base)?;
        if base.scheme() != "https" || !matches!(base.host_str(), Some("127.0.0.1" | "[::1]")) {
            return Err(Error::AuthUnavailable);
        }
        let cert = reqwest::Certificate::from_pem(pem).map_err(|_| Error::AuthUnavailable)?;
        Self::from_builder(base, Client::builder().no_proxy().tls_certs_only([cert]))
    }

    fn from_builder(base: Url, builder: ClientBuilder) -> Result<Self> {
        let http = builder
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(12))
            .build()
            .map_err(|_| Error::AuthUnavailable)?;
        Ok(Self { base, http })
    }

    pub(super) fn origin(&self) -> &str {
        self.base.as_str()
    }

    fn request(&self, method: reqwest::Method, route: &str) -> Result<RequestBuilder> {
        let url = self.base.join(route).map_err(|_| Error::AuthProtocol)?;
        Ok(self.http.request(method, url))
    }

    fn post(&self, route: &str, body: &impl Serialize) -> Result<RequestBuilder> {
        let bytes = serde_json::to_vec(body).map_err(|_| Error::AuthProtocol)?;
        Ok(self
            .request(reqwest::Method::POST, route)?
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes))
    }

    fn get(&self, route: &str, access: &SecretString) -> Result<Response> {
        send(
            self.request(reqwest::Method::GET, route)?
                .bearer_auth(access.expose_secret()),
        )
    }

    pub(super) fn login(&self, login: &str, password: &SecretString) -> Result<LoginResult> {
        if login.is_empty()
            || login.len() > 254
            || password.expose_secret().is_empty()
            || password.expose_secret().len() > 72
        {
            return Err(Error::InvalidInput {
                field: "credentials",
            });
        }
        let result: LoginResult = read(
            send(self.post(
                "api/auth/login",
                &serde_json::json!({
                    "client": "launcher", "login": login, "password": password.expose_secret()
                }),
            )?)
            .map_err(login_error)?,
        )?;
        result.user.validate()?;
        result.tokens.validate()?;
        Ok(result)
    }

    pub(super) fn register(
        &self,
        nickname: &str,
        password: &SecretString,
        email: &str,
        invite: &str,
    ) -> Result<()> {
        if !(3..=50).contains(&nickname.len())
            || !(8..=72).contains(&password.expose_secret().len())
            || email.len() > 254
            || !(1..=20).contains(&invite.len())
        {
            return Err(Error::InvalidInput {
                field: "registration",
            });
        }
        send(self.post("api/auth/register", &serde_json::json!({
            "nickname": nickname, "password": password.expose_secret(), "email": email, "invitation_code": invite
        }))?).map_err(login_error)?;
        Ok(())
    }

    pub(super) fn refresh(&self, refresh: &SecretString) -> Result<TokenPair> {
        let result: TokenPair = read(send(self.post(
            "api/auth/refresh",
            &serde_json::json!({
                "client": "launcher", "refreshToken": refresh.expose_secret()
            }),
        )?)?)?;
        result.validate()?;
        Ok(result)
    }

    pub(super) fn account(&self, access: &SecretString) -> Result<Account> {
        #[derive(Deserialize)]
        struct Envelope {
            user: Account,
        }
        let result: Envelope = read(self.get("api/auth/profile", access)?)?;
        result.user.validate()?;
        Ok(result.user)
    }

    pub(super) fn profiles(&self, access: &SecretString) -> Result<Vec<GameProfile>> {
        #[derive(Deserialize)]
        struct Envelope {
            profiles: Vec<GameProfile>,
        }
        let result: Envelope = read(self.get("api/auth/game-profiles", access)?)?;
        let mut ids = std::collections::HashSet::new();
        for p in &result.profiles {
            p.validate()?;
            if !ids.insert(p.id) {
                return Err(Error::AuthProtocol);
            }
        }
        Ok(result.profiles)
    }

    pub(super) fn launch(
        &self,
        access: &SecretString,
        profile: &GameProfile,
    ) -> Result<(GameToken, Url)> {
        let response = send(
            self.post(
                "api/auth/game-token",
                &serde_json::json!({"profileId": profile.id}),
            )?
            .bearer_auth(access.expose_secret()),
        )?;
        let result: GameToken = read(response)?;
        result.profile.validate()?;
        let expected_path = format!("/yggdrasil/{}/", profile.realm);
        if result.profile.id != profile.id
            || result.profile.realm != profile.realm
            || result.profile.nickname != profile.nickname
            || result.injector_path != expected_path
            || !valid_token(&result.access_token, "mc_")
        {
            return Err(Error::AuthProtocol);
        }
        let root = self
            .base
            .join(&expected_path)
            .map_err(|_| Error::AuthProtocol)?;
        Ok((result, root))
    }

    pub(super) fn logout(&self, refresh: &SecretString) -> Result<()> {
        send(self.post(
            "api/auth/logout",
            &serde_json::json!({
                "client": "launcher", "refreshToken": refresh.expose_secret()
            }),
        )?)?;
        Ok(())
    }
}

fn parse_origin(base: &str) -> Result<Url> {
    let base = Url::parse(base).map_err(|_| Error::AuthUnavailable)?;
    let loopback = cfg!(debug_assertions)
        && base.scheme() == "http"
        && matches!(base.host_str(), Some("127.0.0.1" | "[::1]"));
    if (base.scheme() != "https" && !loopback)
        || base.host().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || base.path() != "/"
    {
        return Err(Error::AuthUnavailable);
    }
    Ok(base)
}

fn login_error(error: Error) -> Error {
    match error {
        Error::AuthenticationRequired => Error::AuthRejected,
        other => other,
    }
}

fn send(request: RequestBuilder) -> Result<Response> {
    let response = request.send().map_err(|_| Error::AuthNetwork)?;
    match response.status() {
        status if status.is_success() => Ok(response),
        StatusCode::UNAUTHORIZED => Err(Error::AuthenticationRequired),
        StatusCode::TOO_MANY_REQUESTS => Err(Error::AuthThrottled),
        StatusCode::NOT_FOUND => Err(Error::GameProfileUnavailable),
        StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN | StatusCode::CONFLICT => {
            Err(Error::AuthRejected)
        }
        status if status.is_server_error() => Err(Error::AuthNetwork),
        _ => Err(Error::AuthProtocol),
    }
}

fn read<T: DeserializeOwned>(response: Response) -> Result<T> {
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::AuthNetwork)?;
    if bytes.len() as u64 > LIMIT {
        return Err(Error::AuthProtocol);
    }
    serde_json::from_slice(&bytes).map_err(|_| Error::AuthProtocol)
}
