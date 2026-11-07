//! Native OS credentials on one dedicated thread (some stores require ordered thread access).
use launcher_core::{
    auth::SecretString,
    core::{LauncherError as Error, Result},
    storage::CredentialStore,
};
use std::sync::{OnceLock, mpsc};
#[derive(Debug, Default)]
pub(crate) struct NativeCredentialStore {
    worker: OnceLock<Option<mpsc::SyncSender<Request>>>,
}
#[derive(Debug)]
enum Request {
    Read(mpsc::SyncSender<Result<Option<SecretString>>>),
    Write(SecretString, mpsc::SyncSender<Result<()>>),
    Delete(mpsc::SyncSender<Result<()>>),
}
fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new("dev.launcher.desktop", "ely-by/current")
        .map_err(|_| Error::CredentialStoreUnavailable)
}
fn read() -> Result<Option<SecretString>> {
    match entry()?.get_password() {
        Ok(s) => Ok(Some(SecretString::new(s))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(Error::CredentialStoreUnavailable),
    }
}
fn write(secret: &SecretString) -> Result<()> {
    entry()?
        .set_password(secret.expose_secret())
        .map_err(|_| Error::CredentialStoreUnavailable)
}
fn delete() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(Error::CredentialStoreUnavailable),
    }
}
impl NativeCredentialStore {
    fn sender(&self, reference: &str) -> Result<&mpsc::SyncSender<Request>> {
        if reference != "ely-by/current" {
            return Err(Error::AuthUnavailable);
        }
        self.worker
            .get_or_init(|| {
                let (tx, rx) = mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .name("launcher-credentials".into())
                    .spawn(move || {
                        for request in rx {
                            match request {
                                Request::Read(reply) => {
                                    let _ = reply.send(read());
                                }
                                Request::Write(secret, reply) => {
                                    let _ = reply.send(write(&secret));
                                }
                                Request::Delete(reply) => {
                                    let _ = reply.send(delete());
                                }
                            }
                        }
                    })
                    .ok()
                    .map(|_| tx)
            })
            .as_ref()
            .ok_or(Error::CredentialStoreUnavailable)
    }
}
impl CredentialStore for NativeCredentialStore {
    fn read(&self, reference: &str) -> Result<Option<SecretString>> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender(reference)?
            .send(Request::Read(tx))
            .map_err(|_| Error::CredentialStoreUnavailable)?;
        rx.recv().map_err(|_| Error::CredentialStoreUnavailable)?
    }
    fn write(&self, reference: &str, secret: &SecretString) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender(reference)?
            .send(Request::Write(
                SecretString::new(secret.expose_secret().into()),
                tx,
            ))
            .map_err(|_| Error::CredentialStoreUnavailable)?;
        rx.recv().map_err(|_| Error::CredentialStoreUnavailable)?
    }
    fn delete(&self, reference: &str) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.sender(reference)?
            .send(Request::Delete(tx))
            .map_err(|_| Error::CredentialStoreUnavailable)?;
        rx.recv().map_err(|_| Error::CredentialStoreUnavailable)?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foreign_references_are_rejected_before_accessing_the_os_store() {
        assert!(matches!(
            NativeCredentialStore::default().read("another-app/password"),
            Err(Error::AuthUnavailable)
        ));
    }
}
