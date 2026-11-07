use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
#[derive(Clone)]
enum Reply {
    Session(&'static str),
    Challenge,
    Status(u16, &'static str),
    WrongClient,
    Huge,
}
struct Fixture {
    base: String,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new(script: Vec<(&'static str, Reply)>) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let base = format!("http://{}", listener.local_addr()?);
        let task = thread::spawn(move || {
            for (action, reply) in script {
                let (mut stream, _) = listener
                    .accept()
                    .unwrap_or_else(|e| panic!("fixture accept: {e}"));
                stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                let mut bytes = vec![];
                let mut byte = [0u8; 1];
                while !bytes.ends_with(b"\r\n\r\n") {
                    stream
                        .read_exact(&mut byte)
                        .unwrap_or_else(|e| panic!("fixture headers: {e}"));
                    bytes.push(byte[0]);
                    assert!(bytes.len() < 16384);
                }
                let header = String::from_utf8_lossy(&bytes);
                assert!(header.starts_with(&format!("POST /auth/{action} HTTP/1.1")));
                let length = header
                    .lines()
                    .find_map(|s| {
                        s.split_once(':')
                            .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or_default();
                assert!(length < 16384);
                let mut body = vec![0; length];
                stream
                    .read_exact(&mut body)
                    .unwrap_or_else(|e| panic!("fixture body: {e}"));
                let json: serde_json::Value =
                    serde_json::from_slice(&body).unwrap_or_else(|e| panic!("fixture JSON: {e}"));
                if action == "authenticate" {
                    assert_eq!(json["username"], "fixture@example.test");
                    assert!(
                        json["password"]
                            .as_str()
                            .is_some_and(|p| p.starts_with("fixture-password"))
                    );
                    assert_eq!(json["requestUser"], false);
                }
                let(status,body)=match reply{
   Reply::Session(token)=>(200,serde_json::json!({"accessToken":token,"clientToken":json["clientToken"],"selectedProfile":{"id":"12345678123442348123123456789abc","name":"ElyPlayer"}}).to_string()),
   Reply::WrongClient=>(200,serde_json::json!({"accessToken":"fixture-token","clientToken":"wrong","selectedProfile":{"id":"12345678123442348123123456789abc","name":"ElyPlayer"}}).to_string()),
   Reply::Challenge=>(401,"{\"error\":\"ForbiddenOperationException\",\"errorMessage\":\"Account protected with two factor auth.\"}".into()),
   Reply::Status(status,body)=>(status,body.into()),Reply::Huge=>(200,"x".repeat(128*1024+1)),
  };
                let response = format!(
                    "HTTP/1.1 {status} fixture\r\nContent-Length: {}\r\nConnection: close\r\nLocation: https://example.com/capture\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Ok(Self {
            base,
            thread: Some(task),
        })
    }
    fn provider(&self) -> ElyByAuthProvider {
        ElyByAuthProvider {
            test_base: Some(self.base.clone()),
            ..Default::default()
        }
    }
    fn finish(mut self) -> Result<()> {
        self.thread
            .take()
            .ok_or(Error::AuthProtocol)?
            .join()
            .map_err(|_| Error::AuthProtocol)
    }
}
fn password() -> SecretString {
    SecretString::new("fixture-password".into())
}
fn authenticate(provider: &ElyByAuthProvider) -> Result<Session> {
    let result = provider.authenticate(AuthInput::Password {
        login: "fixture@example.test".into(),
        password: password(),
        totp: None,
    })?;
    match result {
        AuthOutcome::Authenticated(s) => Ok(s),
        _ => Err(Error::AuthProtocol),
    }
}
#[test]
fn authenticate_rotate_validate_expired_and_empty_invalidate() -> Result<()> {
    let f = Fixture::new(vec![
        ("authenticate", Reply::Session("first-fixture-token")),
        ("validate", Reply::Status(401, "")),
        ("refresh", Reply::Session("rotated-fixture-token")),
        ("validate", Reply::Status(200, "")),
        ("invalidate", Reply::Status(200, "")),
    ])?;
    let provider = f.provider();
    let old = authenticate(&provider)?;
    assert_eq!(old.profile.name, "ElyPlayer");
    assert_eq!(
        provider.validate(&old)?,
        SessionValidity::ReauthenticationRequired
    );
    let fresh = provider.refresh(&old)?;
    assert_ne!(
        old.access_token.expose_secret(),
        fresh.access_token.expose_secret()
    );
    assert_eq!(
        fresh.client_token.as_ref().map(SecretString::expose_secret),
        old.client_token.as_ref().map(SecretString::expose_secret)
    );
    assert_eq!(provider.validate(&fresh)?, SessionValidity::Valid);
    assert!(provider.launch_identity(&fresh)?.requires_ely_by_agent);
    provider.logout(&fresh)?;
    f.finish()
}
#[test]
fn totp_challenge_retry_cancel_and_expiration_remain_backend_owned() -> Result<()> {
    let f = Fixture::new(vec![
        ("authenticate", Reply::Challenge),
        ("authenticate", Reply::Status(401, "rejected")),
        ("authenticate", Reply::Session("totp-fixture-token")),
        ("authenticate", Reply::Challenge),
        ("authenticate", Reply::Challenge),
    ])?;
    let mut service = AuthService::with_test_provider(f.provider());
    let challenged = service.login_password(0, "fixture@example.test".into(), password())?;
    let id = challenged.challenge.ok_or(Error::AuthProtocol)?.id;
    assert!(service.snapshot().account.is_none());
    assert!(
        !serde_json::to_string(&service.snapshot())
            .map_err(|_| Error::AuthProtocol)?
            .contains("fixture-password")
    );
    assert!(matches!(
        service.continue_login(1, Uuid::new_v4(), SecretString::new("123456".into())),
        Err(Error::AuthChallengeExpired)
    ));
    assert!(matches!(
        service.continue_login(1, id, SecretString::new("111111".into())),
        Err(Error::AuthRejected)
    ));
    assert!(service.snapshot().challenge.is_some());
    assert_eq!(
        service
            .continue_login(1, id, SecretString::new("123456".into()))?
            .account
            .ok_or(Error::AuthProtocol)?
            .player_name,
        "ElyPlayer"
    );
    let c = service.login_password(2, "fixture@example.test".into(), password())?;
    let id = c.challenge.ok_or(Error::AuthProtocol)?.id;
    service.cancel_login(3)?;
    assert!(matches!(
        service.continue_login(4, id, SecretString::new("123456".into())),
        Err(Error::AuthChallengeExpired)
    ));
    let c = service.login_password(4, "fixture@example.test".into(), password())?;
    let id = c.challenge.ok_or(Error::AuthProtocol)?.id;
    service.expire_test_challenge();
    assert!(matches!(
        service.continue_login(5, id, SecretString::new("123456".into())),
        Err(Error::AuthChallengeExpired)
    ));
    assert!(service.snapshot().challenge.is_none());
    f.finish()
}
#[test]
fn invalid_credentials_network_redirect_and_malformed_body_are_distinct() -> Result<()> {
    for (reply, expected) in [
        (
            Reply::Status(401, "untrusted error with fixture-password"),
            "rejected",
        ),
        (Reply::Status(503, "internal secret"), "network"),
        (Reply::Status(302, ""), "protocol"),
        (Reply::Status(200, "not JSON"), "protocol"),
        (Reply::WrongClient, "protocol"),
        (Reply::Huge, "protocol"),
    ] {
        let f = Fixture::new(vec![("authenticate", reply)])?;
        let result = authenticate(&f.provider());
        assert!(match expected {
            "rejected" => matches!(result, Err(Error::AuthRejected)),
            "network" => matches!(result, Err(Error::AuthNetwork)),
            _ => matches!(result, Err(Error::AuthProtocol)),
        });
        assert!(!format!("{result:?}").contains("fixture-password"));
        f.finish()?;
    }
    Ok(())
}
#[test]
fn failed_refresh_preserves_session_and_failed_revoke_still_logs_out() -> Result<()> {
    let f = Fixture::new(vec![
        ("authenticate", Reply::Session("retained-token")),
        ("refresh", Reply::Status(503, "")),
        ("invalidate", Reply::Status(503, "")),
    ])?;
    let mut s = AuthService::with_test_provider(f.provider());
    s.login_password(0, "fixture@example.test".into(), password())?;
    assert!(matches!(s.refresh(1), Err(Error::AuthNetwork)));
    assert!(s.snapshot().account.is_some());
    assert_eq!(s.snapshot().revision, 1);
    let result = s.logout(1)?;
    assert!(result.account.is_none());
    assert_eq!(result.notice, Some("remote_logout_failed"));
    assert!(matches!(
        s.launch_identity(),
        Err(Error::AuthenticationRequired)
    ));
    f.finish()
}

#[test]
fn launch_boundary_validates_refreshes_and_never_falls_back_on_network_error() -> Result<()> {
    let f = Fixture::new(vec![
        ("authenticate", Reply::Session("first-token")),
        ("validate", Reply::Status(401, "")),
        ("refresh", Reply::Session("new-token")),
        ("validate", Reply::Status(200, "")),
        ("validate", Reply::Status(503, "")),
    ])?;
    let mut service = AuthService::with_test_provider(f.provider());
    service.login_password(0, "fixture@example.test".into(), password())?;
    let identity = service.launch_identity()?;
    assert_eq!(identity.access_token.expose_secret(), "new-token");
    assert!(identity.requires_ely_by_agent);
    assert_eq!(service.snapshot().revision, 2);
    assert_eq!(
        service.launch_identity()?.access_token.expose_secret(),
        "new-token"
    );
    assert!(matches!(service.launch_identity(), Err(Error::AuthNetwork)));
    assert_eq!(
        service
            .snapshot()
            .account
            .ok_or(Error::AuthProtocol)?
            .provider_id,
        "ely-by"
    );
    f.finish()
}

#[test]
fn totp_attempt_limit_drops_challenge_and_deadline_cleanup_drops_idle_secret() -> Result<()> {
    let mut script = vec![("authenticate", Reply::Challenge)];
    script.extend((0..5).map(|_| ("authenticate", Reply::Status(401, "rejected"))));
    script.push(("authenticate", Reply::Challenge));
    let f = Fixture::new(script)?;
    let mut service = AuthService::with_test_provider(f.provider());
    let id = service
        .login_password(0, "fixture@example.test".into(), password())?
        .challenge
        .ok_or(Error::AuthProtocol)?
        .id;
    for _ in 0..5 {
        assert!(matches!(
            service.continue_login(1, id, SecretString::new("111111".into())),
            Err(Error::AuthRejected)
        ));
    }
    assert!(service.snapshot().challenge.is_none());
    assert!(matches!(
        service.continue_login(2, id, SecretString::new("111111".into())),
        Err(Error::AuthChallengeExpired)
    ));
    service.login_password(2, "fixture@example.test".into(), password())?;
    service.expire_test_challenge();
    service.expire_challenge();
    assert!(service.snapshot().challenge.is_none());
    assert!(!format!("{service:?}").contains("fixture-password"));
    f.finish()
}
