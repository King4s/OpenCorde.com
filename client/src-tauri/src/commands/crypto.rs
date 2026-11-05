//! Durable account-scoped MLS commands. The renderer receives public relay
//! operations and channel handles; provider snapshots remain encrypted locally.
#[path = "crypto_vault.rs"]
pub mod vault;

use opencorde_crypto::durable_store::{Account, EncryptedStore, KeyMaterial, KeyProvider};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};
use vault::{Event, MemberPackage, Prepared, Vault, Welcome};

struct OsKeys;
fn account_name(account: &Account) -> String {
    vault::hex(&Sha256::digest(
        format!("{}\0{}", account.server_url, account.user_id).as_bytes(),
    ))
}
impl KeyProvider for OsKeys {
    fn load(&self, account: &Account) -> Result<Option<KeyMaterial>, String> {
        let entry = keyring::Entry::new("opencorde-mls-05-v2", &account_name(account))
            .map_err(|_| "OS credential store unavailable; encryption remains locked")?;
        match entry.get_password() {
            Ok(record) => serde_json::from_str(&record)
                .map(Some)
                .map_err(|_| "invalid protected MLS key record".into()),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => {
                Err("OS credential store unavailable or locked; encryption remains locked".into())
            }
        }
    }
    fn save(&self, account: &Account, material: &KeyMaterial) -> Result<(), String> {
        let entry = keyring::Entry::new("opencorde-mls-05-v2", &account_name(account))
            .map_err(|_| "OS credential store unavailable; encryption remains locked")?;
        let record =
            serde_json::to_string(material).map_err(|_| "cannot encode protected MLS key")?;
        entry
            .set_password(&record)
            .map_err(|_| "cannot save OS-protected MLS key")?;
        if entry
            .get_password()
            .map_err(|_| "cannot verify OS-protected MLS key")?
            != record
        {
            return Err("OS-protected MLS key verification failed".into());
        }
        Ok(())
    }
}
struct Session {
    id: String,
    store: EncryptedStore<Vault>,
}
pub struct CryptoState {
    session: Mutex<Option<Session>>,
    keys: Arc<dyn KeyProvider>,
    directory: Option<PathBuf>,
}
impl Default for CryptoState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            keys: Arc::new(OsKeys),
            directory: None,
        }
    }
}
impl CryptoState {
    #[cfg(test)]
    #[allow(dead_code)] // Used by the production-handler integration test module.
    pub fn isolated(directory: PathBuf, keys: Arc<dyn KeyProvider>) -> Self {
        Self {
            session: Mutex::new(None),
            keys,
            directory: Some(directory),
        }
    }
    #[cfg(test)]
    #[allow(dead_code)] // Used by the production-handler integration test module.
    pub fn isolated_os_keys(directory: PathBuf) -> Self {
        Self {
            session: Mutex::new(None),
            keys: Arc::new(OsKeys),
            directory: Some(directory),
        }
    }
    pub fn with_session<T>(
        &self,
        id: &str,
        operation: impl FnOnce(&mut EncryptedStore<Vault>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| "crypto session lock poisoned; restart the app")?;
        let session = session
            .as_mut()
            .filter(|session| session.id == id)
            .ok_or("crypto session is locked or belongs to another account; sign in again")?;
        operation(&mut session.store)
    }
}
#[derive(Serialize)]
pub struct ChannelInfo {
    pub channel_id: String,
    pub sequence: String,
    pub epoch: String,
    pub active: bool,
    pub welcome_ack: Option<String>,
}
#[derive(Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub device_id: String,
    pub key_package_hex: Option<String>,
    pub channels: Vec<ChannelInfo>,
    pub pending: Option<Prepared>,
    pub retry_channel: Option<String>,
    pub recovered_drafts: std::collections::BTreeMap<String, String>,
}
fn info(id: &str, store: &EncryptedStore<Vault>) -> Result<SessionInfo, String> {
    let data = store.data()?;
    Ok(SessionInfo {
        session_id: id.into(),
        device_id: store.device_id().into(),
        key_package_hex: data.key_package_hex.clone(),
        pending: data.pending(),
        retry_channel: data.retry_channel(),
        recovered_drafts: data.recovered_drafts.clone(),
        channels: data
            .channels
            .iter()
            .map(|(id, channel)| ChannelInfo {
                channel_id: id.clone(),
                sequence: channel.cursor.to_string(),
                epoch: channel.epoch.to_string(),
                active: channel.active,
                welcome_ack: channel.welcome_ack.clone(),
            })
            .collect(),
    })
}
fn normalize_server(value: &str) -> Result<String, String> {
    let mut url = tauri::Url::parse(value).map_err(|_| "invalid crypto server address")?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid crypto server scope".into());
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn crypto_init<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    server_url: String,
    user_id: String,
    state: State<'_, CryptoState>,
) -> Result<SessionInfo, String> {
    let user = user_id.parse::<i64>().map_err(|_| "invalid user id")?;
    if user <= 0 || user.to_string() != user_id {
        return Err("invalid user id".into());
    }
    let account = Account {
        server_url: normalize_server(&server_url)?,
        user_id,
    };
    let mut session = state
        .session
        .lock()
        .map_err(|_| "crypto session lock poisoned")?;
    if let Some(active) = session.as_ref() {
        if active.store.account() == &account {
            return info(&active.id, &active.store);
        }
    }
    // Drop the old account owner before attempting to unlock another namespace.
    *session = None;
    let directory = match &state.directory {
        Some(path) => path.clone(),
        None => app
            .path()
            .app_local_data_dir()
            .map_err(|_| "native app data directory unavailable")?
            .join("mls-v2"),
    }
    .join(account_name(&account));
    let server_url = account.server_url.clone();
    let mut store = EncryptedStore::open(&directory, account, state.keys.as_ref(), || {
        Vault::fresh(user, server_url.clone())
    })?;
    store.data()?.validate(user, &server_url)?;
    if store.data()?.pending().is_none() && store.data()?.retry_channel().is_none() {
        store.transact(|vault| vault.ensure_key_package())?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    let result = info(&id, &store)?;
    *session = Some(Session { id, store });
    Ok(result)
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_lock(session_id: String, state: State<'_, CryptoState>) -> Result<(), String> {
    let mut active = state
        .session
        .lock()
        .map_err(|_| "crypto session lock poisoned")?;
    if active
        .as_ref()
        .is_some_and(|session| session.id == session_id)
    {
        *active = None;
    }
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_status(
    session_id: String,
    state: State<'_, CryptoState>,
) -> Result<SessionInfo, String> {
    state.with_session(&session_id, |store| info(&session_id, store))
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_key_package(
    session_id: String,
    confirmed_hex: Option<String>,
    state: State<'_, CryptoState>,
) -> Result<SessionInfo, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| match confirmed_hex {
            Some(ref bytes) => vault.confirm_key_package(bytes),
            None => vault.ensure_key_package(),
        })?;
        info(&session_id, store)
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_prepare_init(
    session_id: String,
    channel_id: String,
    allow_legacy_rejoin: bool,
    state: State<'_, CryptoState>,
) -> Result<Prepared, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.prepare_init(&channel_id, allow_legacy_rejoin))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_prepare_message(
    session_id: String,
    channel_id: String,
    plaintext: String,
    reply_to_id: Option<String>,
    attachments: Vec<serde_json::Value>,
    state: State<'_, CryptoState>,
) -> Result<Prepared, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| {
            vault.prepare_message(&channel_id, plaintext, reply_to_id, attachments)
        })
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_prepare_members(
    session_id: String,
    channel_id: String,
    added: Vec<MemberPackage>,
    removed: Option<String>,
    state: State<'_, CryptoState>,
) -> Result<Prepared, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.prepare_members(&channel_id, added, removed))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_accept_event(
    session_id: String,
    event: Event,
    state: State<'_, CryptoState>,
) -> Result<(), String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.accept(&event))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_reject_conflict(
    session_id: String,
    operation_id: String,
    state: State<'_, CryptoState>,
) -> Result<(), String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.reject_conflict(&operation_id))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_apply_events(
    session_id: String,
    channel_id: String,
    events: Vec<Event>,
    state: State<'_, CryptoState>,
) -> Result<(), String> {
    if events.len() > 100 {
        return Err("too many events in one batch".into());
    }
    state.with_session(&session_id, |store| {
        store.transact(|vault| {
            for event in &events {
                vault.apply(&channel_id, event)?;
            }
            Ok(())
        })
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_join(
    session_id: String,
    channel_id: String,
    welcome: Welcome,
    state: State<'_, CryptoState>,
) -> Result<(), String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.join(&channel_id, &welcome))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_ack_welcome(
    session_id: String,
    channel_id: String,
    operation_id: String,
    state: State<'_, CryptoState>,
) -> Result<(), String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.ack_welcome(&channel_id, &operation_id))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_cached_message(
    session_id: String,
    channel_id: String,
    message_id: String,
    ciphertext_hex: String,
    state: State<'_, CryptoState>,
) -> Result<Option<String>, String> {
    state.with_session(&session_id, |store| {
        store
            .data()?
            .message(&channel_id, &message_id, &ciphertext_hex)
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_export_voice_key(
    session_id: String,
    channel_id: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    state.with_session(&session_id, |store| {
        store
            .data()?
            .voice_key(&channel_id)
            .map(|bytes| vault::hex(&bytes))
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn crypto_retry_operation(
    session_id: String,
    state: State<'_, CryptoState>,
) -> Result<Prepared, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.retry_operation())
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn crypto_request_rejoin(
    session_id: String,
    channel_id: String,
    state: State<'_, CryptoState>,
) -> Result<SessionInfo, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.request_rejoin(&channel_id))?;
        info(&session_id, store)
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn crypto_cancel_retry(
    session_id: String,
    channel_id: String,
    state: State<'_, CryptoState>,
) -> Result<SessionInfo, String> {
    state.with_session(&session_id, |store| {
        store.transact(|vault| vault.cancel_retry(&channel_id))?;
        info(&session_id, store)
    })
}
