use super::{
    Cancellation,
    engine::InstallEngine,
    metadata::{Download, Resource},
    network::Http,
};
use crate::{
    core::{LauncherError as Error, OperationStage, Result},
    profiles::SettingsPolicy,
    storage::{AppPaths, WorkspaceService},
};
use sha1::{Digest, Sha1};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
#[derive(Debug)]
struct Server {
    base: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn new(body: Vec<u8>, bad_first: bool) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let base = format!("http://{}/", listener.local_addr()?);
        listener.set_nonblocking(true)?;
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let c = count.clone();
        let s = stop.clone();
        let thread = std::thread::spawn(move || {
            while !s.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let mut request = [0; 4096];
                        let mut received = 0;
                        while received < request.len() {
                            match stream.read(&mut request[received..]) {
                                Ok(0) | Err(_) => break,
                                Ok(size) => received += size,
                            }
                            if request[..received].windows(4).any(|v| v == b"\r\n\r\n") {
                                break;
                            }
                        }
                        if !request[..received].windows(4).any(|v| v == b"\r\n\r\n") {
                            continue;
                        }
                        let index = c.fetch_add(1, Ordering::AcqRel);
                        let body = if bad_first && index == 0 {
                            vec![b'x'; body.len()]
                        } else {
                            body.clone()
                        };
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let mut response = response.into_bytes();
                        response.extend(body);
                        let _ = stream.write_all(&response);
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Ok(Self {
            base,
            count,
            stop,
            thread: Some(thread),
        })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn store(
    base: &std::path::Path,
) -> Result<(
    WorkspaceService,
    Arc<crate::storage::ResourceStore>,
    crate::profiles::ProfileId,
)> {
    let service = WorkspaceService::open(
        AppPaths {
            data: base.join("data"),
            configuration: base.join("config"),
            cache: base.join("cache"),
            logs: base.join("logs"),
        },
        SettingsPolicy::new(8192)?,
    )?;
    let snapshot = service.snapshot()?;
    let store = service.installation_store(snapshot.root_id, snapshot.active_profile_id)?;
    Ok((service, store, snapshot.active_profile_id))
}
#[test]
fn bad_hash_retries_valid_reuse_is_offline_and_same_size_corruption_repairs()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let body = b"correct artifact".to_vec();
    let server = Server::new(body.clone(), true)?;
    let temp = tempfile::TempDir::new()?;
    let (_service, store, id) = store(temp.path())?;
    let mut engine = InstallEngine::new(store.clone(), id, Cancellation::default())?;
    engine.http = Http::test(&server.base)?;
    let resource = Resource::new(
        "shared/libraries/test.jar".into(),
        Download {
            url: server.base.clone(),
            sha1: format!("{:x}", Sha1::digest(&body)),
            size: body.len() as u64,
        },
    )?;
    let transfer = std::cell::Cell::new(None);
    engine.ensure_with_transfer(&resource, |stage| transfer.set(Some(stage)))?;
    assert_eq!(transfer.get(), Some(OperationStage::Downloading));
    assert_eq!(server.count.load(Ordering::Acquire), 2);
    transfer.set(None);
    engine.ensure_with_transfer(&resource, |stage| transfer.set(Some(stage)))?;
    assert_eq!(transfer.get(), None);
    assert_eq!(server.count.load(Ordering::Acquire), 2);
    std::fs::write(store.path(&resource.path)?, vec![b'z'; body.len()])?;
    engine.ensure_with_transfer(&resource, |stage| transfer.set(Some(stage)))?;
    assert_eq!(transfer.get(), Some(OperationStage::Repairing));
    assert_eq!(server.count.load(Ordering::Acquire), 3);
    assert_eq!(std::fs::read(store.path(&resource.path)?)?, body);
    assert!(
        std::fs::read_dir(store.path("shared/libraries")?)?
            .all(|e| e.is_ok_and(|e| !e.file_name().to_string_lossy().ends_with(".part")))
    );
    Ok(())
}
#[test]
fn cancellation_and_oversize_never_replace_a_valid_file()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::TempDir::new()?;
    let (_service, store, _) = store(temp.path())?;
    let body = b"valid";
    let resource = Resource::new(
        "shared/libraries/file.jar".into(),
        Download {
            url: "https://libraries.minecraft.net/file.jar".into(),
            sha1: format!("{:x}", Sha1::digest(body)),
            size: body.len() as u64,
        },
    )?;
    store.publish(&resource, &body[..], &Cancellation::default(), |_| {})?;
    assert!(matches!(
        store.publish(
            &resource,
            &b"longer invalid body"[..],
            &Cancellation::default(),
            |_| {}
        ),
        Err(Error::Integrity)
    ));
    let cancel = Cancellation::default();
    cancel.cancel();
    assert!(matches!(
        store.publish(&resource, &body[..], &cancel, |_| {}),
        Err(Error::Cancelled)
    ));
    assert_eq!(std::fs::read(store.path(&resource.path)?)?, body);
    Ok(())
}
#[test]
fn upstream_policy_rejects_credentials_downgrades_and_arbitrary_hosts() -> Result<()> {
    let http = Http::official()?;
    for url in [
        "http://libraries.minecraft.net/file",
        "https://libraries.minecraft.net.evil.test/file",
        "https://user:password@libraries.minecraft.net/file",
        "https://127.0.0.1/file",
        "https://libraries.minecraft.net:444/file",
        "file:///tmp/private",
    ] {
        assert!(
            http.validate(&reqwest::Url::parse(url).map_err(|_| Error::Metadata)?)
                .is_err()
        );
    }
    Ok(())
}

#[test]
fn cancelling_a_streamed_http_body_removes_the_partial()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let body = vec![42u8; 256 * 1024];
    let server = Server::new(body.clone(), false)?;
    let temp = tempfile::TempDir::new()?;
    let (_service, store, _) = store(temp.path())?;
    let http = Http::test(&server.base)?;
    let cancel = Cancellation::default();
    let resource = Resource::new(
        "shared/assets/objects/streamed".into(),
        Download {
            url: server.base.clone(),
            sha1: format!("{:x}", Sha1::digest(&body)),
            size: body.len() as u64,
        },
    )?;
    let response = http.response(&server.base, &cancel)?;
    let outcome = store.publish(&resource, response, &cancel, |_| cancel.cancel());
    assert!(matches!(outcome, Err(Error::Cancelled)));
    assert!(!store.path(&resource.path)?.exists());
    assert!(
        std::fs::read_dir(store.path("shared/assets/objects")?)?
            .next()
            .is_none()
    );
    Ok(())
}
