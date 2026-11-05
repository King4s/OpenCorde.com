//! Authenticated whole-vault snapshots. An OS file lock covers the lifetime of
//! each owner. Results may leave native code only after replace() succeeds.
use aes_gcm::{aead::{Aead, KeyInit, Payload}, Aes256Gcm, Nonce};
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::{Path, PathBuf}};
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8] = b"OCMLS05VAULT\x01";
const MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub server_url: String,
    pub user_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyMaterial {
    pub key: [u8; 32],
    pub device_id: String,
    // A key whose first snapshot was never committed has never released secrets.
    pub initialized: bool,
}
impl Drop for KeyMaterial {
    fn drop(&mut self) { self.key.zeroize(); }
}

/// Production implementations must use an OS-protected credential backend.
/// Called while the account's exclusive disk lock is held.
pub trait KeyProvider: Send + Sync {
    fn load(&self, account: &Account) -> Result<Option<KeyMaterial>, String>;
    fn save(&self, account: &Account, material: &KeyMaterial) -> Result<(), String>;
}

// Closing a descriptor alone can leave flock held by a forked child until exec.
// Explicit unlock binds ownership to this guard, including failed open paths.
struct OwnerLock(File);
impl Drop for OwnerLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub struct EncryptedStore<T> {
    data: T,
    account: Account,
    material: KeyMaterial,
    directory: PathBuf,
    _lock: OwnerLock,
    blocked: bool,
    #[cfg(test)]
    pub(crate) crash_phase: Option<&'static str>,
}

impl<T: Serialize + DeserializeOwned + Clone> EncryptedStore<T> {
    pub fn open(
        directory: &Path, account: Account, keys: &dyn KeyProvider,
        initial: impl FnOnce() -> Result<T, String>,
    ) -> Result<Self, String> {
        if account.server_url.is_empty() || account.user_id.parse::<i64>().ok().filter(|id| *id > 0).is_none() {
            return Err("invalid crypto account scope".into());
        }
        create_directory(directory).map_err(|_| "cannot create encrypted vault directory")?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(directory.join("owner.lock")).map_err(|_| "cannot open vault lock")?;
        lock.try_lock().map_err(|_| "crypto vault is already open in another process")?;
        let lock = OwnerLock(lock);
        let path = directory.join("vault.bin");
        let exists = path.try_exists().map_err(|_| "cannot inspect encrypted vault")?;
        let mut material = match keys.load(&account)? {
            Some(material) => material,
            None if exists => return Err("OS encryption key is missing; preserve the vault and restore its key or explicitly rejoin".into()),
            None => {
                let mut key = [0; 32];
                let mut device = [0; 16];
                OsRng.fill_bytes(&mut key);
                OsRng.fill_bytes(&mut device);
                let material = KeyMaterial { key, device_id: hex(&device), initialized: false };
                keys.save(&account, &material)?;
                material
            }
        };
        if material.initialized && !exists {
            return Err("encrypted vault is missing; restore it or explicitly rejoin; existing key was preserved".into());
        }
        let data = if exists {
            let mut bytes = Vec::new();
            File::open(&path).and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut bytes))
                .map_err(|_| "cannot read encrypted vault")?;
            if bytes.len() as u64 > MAX_BYTES || !bytes.starts_with(MAGIC) || bytes.len() < MAGIC.len() + 28 {
                return Err("unsupported or corrupt vault; original data preserved".into());
            }
            let cipher = Aes256Gcm::new_from_slice(&material.key).map_err(|_| "invalid wrapping key")?;
            let plaintext = Zeroizing::new(cipher.decrypt(
                Nonce::from_slice(&bytes[MAGIC.len()..MAGIC.len()+12]),
                Payload { msg: &bytes[MAGIC.len()+12..], aad: &aad(&account, &material.device_id)? },
            ).map_err(|_| "vault authentication failed; account/key mismatch or corruption")?);
            serde_json::from_slice(&plaintext).map_err(|_| "unsupported encrypted vault contents")?
        } else {
            initial()?
        };
        // Only the authenticated committed snapshot is authoritative. A pending
        // write never released its result; its public operation is retried from
        // the committed outbox after reopen, including after an uncertain ACK.
        let pending = directory.join("vault.pending");
        if pending.try_exists().map_err(|_| "cannot inspect pending vault")? {
            if !pending.is_file() {
                return Err("invalid pending vault path; preserve it for recovery".into());
            }
            std::fs::remove_file(&pending).map_err(|_| "cannot recover interrupted vault write")?;
            sync_directory(directory).map_err(|_| "cannot sync recovered vault")?;
        }
        let mut store = Self { data, account, material: material.clone(), directory: directory.into(),
            _lock: lock, blocked: false, #[cfg(test)] crash_phase: None };
        if !exists { store.persist(&store.data.clone())?; }
        if !material.initialized {
            material.initialized = true;
            keys.save(&store.account, &material)?;
            store.material = material;
        }
        Ok(store)
    }

    pub fn data(&self) -> Result<&T, String> {
        if self.blocked { return Err("crypto vault write failed; reopen the account before continuing".into()); }
        Ok(&self.data)
    }
    pub fn device_id(&self) -> &str { &self.material.device_id }
    pub fn account(&self) -> &Account { &self.account }

    pub fn transact<R>(&mut self, operation: impl FnOnce(&mut T) -> Result<R, String>) -> Result<R, String> {
        let mut candidate = self.data()?.clone();
        let result = operation(&mut candidate)?;
        self.blocked = true;
        self.persist(&candidate)?;
        self.data = candidate;
        self.blocked = false;
        Ok(result)
    }

    fn persist(&self, candidate: &T) -> Result<(), String> {
        let plaintext = Zeroizing::new(serde_json::to_vec(candidate).map_err(|_| "cannot encode crypto vault")?);
        if plaintext.len() as u64 > MAX_BYTES - 128 { return Err("crypto vault is full; no operation was sent".into()); }
        let cipher = Aes256Gcm::new_from_slice(&self.material.key).map_err(|_| "invalid wrapping key")?;
        let mut nonce = [0; 12];
        OsRng.fill_bytes(&mut nonce);
        let encrypted = cipher.encrypt(Nonce::from_slice(&nonce), Payload {
            msg: &plaintext, aad: &aad(&self.account, &self.material.device_id)?,
        }).map_err(|_| "cannot encrypt vault")?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let pending = self.directory.join("vault.pending");
        let mut file = options.open(&pending).map_err(|_| "cannot stage crypto vault")?;
        file.write_all(MAGIC).and_then(|_| file.write_all(&nonce)).and_then(|_| file.write_all(&encrypted))
            .map_err(|_| "cannot write crypto vault")?;
        #[cfg(test)] self.crash("write");
        file.sync_all().map_err(|_| "cannot flush crypto vault")?;
        #[cfg(test)] self.crash("file-sync");
        drop(file);
        replace(&pending, &self.directory.join("vault.bin")).map_err(|_| "cannot atomically replace crypto vault")?;
        #[cfg(test)] self.crash("replace");
        sync_directory(&self.directory).map_err(|_| "cannot flush vault directory")?;
        #[cfg(test)] self.crash("directory-sync");
        Ok(())
    }

    #[cfg(test)]
    fn crash(&self, phase: &'static str) {
        if self.crash_phase == Some(phase) { std::process::exit(73); }
    }
}

pub fn hex(bytes: &[u8]) -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() }
fn aad(account: &Account, device: &str) -> Result<Vec<u8>, String> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(serde_json::to_vec(&(account, device)).map_err(|_| "invalid account scope")?);
    Ok(bytes)
}
fn create_directory(path: &Path) -> std::io::Result<()> {
    if path.is_dir() { return Ok(()); }
    if let Some(parent) = path.parent() { create_directory(parent)?; }
    match std::fs::create_dir(path) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && path.is_dir() => (),
        Err(error) => return Err(error),
    }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    if let Some(parent) = path.parent() { sync_directory(parent)?; }
    Ok(())
}
#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> { File::open(path)?.sync_all() }
#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> std::io::Result<()> { Ok(()) }
#[cfg(not(windows))]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> { std::fs::rename(source, destination) }
#[cfg(windows)]
fn replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both NUL-terminated buffers remain valid throughout the call.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(std::io::Error::last_os_error())
    } else { Ok(()) }
}

#[cfg(test)]
mod tests;
