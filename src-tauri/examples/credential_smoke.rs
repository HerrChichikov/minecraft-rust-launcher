//! Explicit native credential test. Uses a fresh test-only service/entry, never production credentials.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = format!(
        "phase-4-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let entry = keyring::Entry::new("dev.launcher.desktop.smoke", &user)?;
    match entry.get_password() {
        Err(keyring::Error::NoEntry) => {}
        Ok(_) => return Err("test entry collision".into()),
        Err(_) => return Err("native credential store unavailable".into()),
    }
    let outcome = (|| {
        entry.set_password("fixture-not-a-real-token")?;
        let matches = entry.get_password()? == "fixture-not-a-real-token";
        if !matches {
            return Err(keyring::Error::NoEntry);
        }
        Ok(())
    })();
    let cleanup = entry.delete_credential();
    if outcome.is_err() || cleanup.is_err() {
        return Err("credential roundtrip or cleanup failed".into());
    }
    if !matches!(entry.get_password(), Err(keyring::Error::NoEntry)) {
        return Err("credential cleanup not confirmed".into());
    }
    println!("native credential store: isolated write/read/delete pass; no real account data used");
    Ok(())
}
