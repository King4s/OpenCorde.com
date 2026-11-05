use super::*;
use std::process::Command;

struct TestKeys(PathBuf);
impl KeyProvider for TestKeys {
    fn load(&self, _: &Account) -> Result<Option<KeyMaterial>, String> {
        match std::fs::read(&self.0) {
            Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn save(&self, _: &Account, value: &KeyMaterial) -> Result<(), String> {
        std::fs::write(&self.0, serde_json::to_vec(value).unwrap()).map_err(|e| e.to_string())
    }
}
fn account() -> Account {
    Account { server_url: "https://example.test/api".into(), user_id: "9007199254740993".into() }
}
fn directory() -> PathBuf {
    let mut random = [0; 16];
    OsRng.fill_bytes(&mut random);
    let path = std::env::temp_dir().join(format!("opencorde-vault-test-{}", hex(&random)));
    std::fs::create_dir(&path).unwrap();
    path
}
fn open(path: &Path) -> Result<EncryptedStore<String>, String> {
    EncryptedStore::open(&path.join("vault"), account(), &TestKeys(path.join("test-key.json")),
        || Ok("committed secret".into()))
}
#[test]
fn restart_integrity_lock_and_missing_key_fail_closed() {
    let path = directory();
    let mut vault = open(&path).unwrap();
    assert!(open(&path).is_err(), "second process owner must not open");
    vault.transact(|value| { *value = "advanced secret".into(); Ok(()) }).unwrap();
    drop(vault);
    let bytes = std::fs::read(path.join("vault/vault.bin")).unwrap();
    assert!(!bytes.windows(b"advanced secret".len()).any(|slice| slice == b"advanced secret"));
    let vault = open(&path).unwrap();
    assert_eq!(vault.data().unwrap(), "advanced secret");
    drop(vault);
    let wrong = Account { server_url: "https://other.test".into(), ..account() };
    assert!(EncryptedStore::<String>::open(&path.join("vault"), wrong, &TestKeys(path.join("test-key.json")),
        || panic!("must never replace existing state")).is_err());
    std::fs::rename(path.join("test-key.json"), path.join("saved-test-key.json")).unwrap();
    assert!(open(&path).is_err());
    assert_eq!(std::fs::read(path.join("vault/vault.bin")).unwrap(), bytes);
    std::fs::rename(path.join("saved-test-key.json"), path.join("test-key.json")).unwrap();
    let mut tampered = bytes.clone();
    *tampered.last_mut().unwrap() ^= 1;
    std::fs::write(path.join("vault/vault.bin"), &tampered).unwrap();
    assert!(open(&path).is_err());
    assert_eq!(std::fs::read(path.join("vault/vault.bin")).unwrap(), tampered);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn failed_write_releases_no_result_and_requires_reopen() {
    let path = directory();
    let mut vault = open(&path).unwrap();
    std::fs::create_dir(path.join("vault/vault.pending")).unwrap();
    assert!(vault.transact(|value| { *value = "unsent ciphertext".into(); Ok("do not release") }).is_err());
    assert!(vault.data().is_err());
    assert!(vault.transact(|_| Ok(())).is_err());
    drop(vault);
    std::fs::remove_dir(path.join("vault/vault.pending")).unwrap();
    assert_eq!(open(&path).unwrap().data().unwrap(), "committed secret");
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn actual_process_crashes_recover_only_committed_snapshot() {
    for phase in ["write", "file-sync", "replace", "directory-sync"] {
        let path = directory();
        drop(open(&path).unwrap());
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "durable_store::tests::crash_worker", "--nocapture"])
            .env("OPENCORDE_VAULT_TEST_DIRECTORY", &path)
            .env("OPENCORDE_VAULT_TEST_PHASE", phase)
            .status().unwrap();
        assert_eq!(status.code(), Some(73));
        let reopened = open(&path).unwrap();
        let expected = if matches!(phase, "write" | "file-sync") { "committed secret" } else { "advanced secret" };
        assert_eq!(reopened.data().unwrap(), expected, "phase {phase}");
        drop(reopened);
        assert!(!path.join("vault/vault.pending").exists());
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn crash_worker() {
    let Some(path) = std::env::var_os("OPENCORDE_VAULT_TEST_DIRECTORY") else { return; };
    let phase = std::env::var("OPENCORDE_VAULT_TEST_PHASE").unwrap();
    let mut vault = open(Path::new(&path)).unwrap();
    vault.crash_phase = Some(match phase.as_str() {
        "write" => "write", "file-sync" => "file-sync",
        "replace" => "replace", "directory-sync" => "directory-sync", _ => panic!("unknown phase"),
    });
    vault.transact(|value| { *value = "advanced secret".into(); Ok(()) }).unwrap();
    panic!("crash hook did not fire");
}
