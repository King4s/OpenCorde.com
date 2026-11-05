//! Native-only MLS state machine. Public relay DTOs never contain snapshots.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use opencorde_crypto::{
    encrypt, group, key_package,
    provider::{PersistentCryptoProvider, ProviderSnapshot},
};
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};

const CACHE_PER_CHANNEL: usize = 1000;
const CACHE_BYTES: usize = 16 * 1024 * 1024;

pub fn hex(bytes: &[u8]) -> String {
    opencorde_crypto::durable_store::hex(bytes)
}
pub fn unhex(value: &str) -> Result<Vec<u8>, String> {
    if !value.is_ascii() || !value.len().is_multiple_of(2) {
        return Err("invalid hex encoding".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).map_err(|_| "invalid hex")?, 16)
                .map_err(|_| "invalid hex".into())
        })
        .collect()
}
fn decode(value: &str) -> Result<Vec<u8>, String> {
    if value.len() > 262144 {
        return Err("relay payload exceeds limit".into());
    }
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "invalid relay encoding".into())
}
fn number(value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<i64>()
        .map_err(|_| "invalid relay sequence or epoch")?;
    if parsed < 0 || parsed.to_string() != value {
        return Err("invalid relay sequence or epoch".into());
    }
    Ok(parsed as u64)
}
fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WelcomeTarget {
    pub user_id: String,
    pub welcome: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operation_id: String,
    pub expected_sequence: String,
    pub epoch: String,
    pub kind: String,
    pub ciphertext: String,
    #[serde(default)]
    pub added: Vec<WelcomeTarget>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<serde_json::Value>,
    #[serde(default)]
    pub allow_legacy_rejoin: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Prepared {
    pub channel_id: String,
    pub operation: Operation,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    pub operation_id: String,
    pub sequence: String,
    pub epoch: String,
    pub sender_id: String,
    pub kind: String,
    pub ciphertext: String,
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub message_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Welcome {
    pub operation_id: String,
    pub sequence: String,
    pub epoch: String,
    pub welcome: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct MemberPackage {
    pub user_id: String,
    pub key_package_hex: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Cached {
    id: String,
    digest: String,
    plaintext: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Channel {
    group: Vec<u8>,
    pub cursor: u64,
    pub epoch: u64,
    pub active: bool,
    cache: VecDeque<Cached>,
    file_keys: BTreeMap<u64, Vec<u8>>,
    pub welcome_ack: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
enum Intent {
    Init {
        channel_id: String,
        legacy: bool,
    },
    Message {
        channel_id: String,
        plaintext: String,
        reply_to_id: Option<String>,
        attachments: Vec<serde_json::Value>,
    },
    Members {
        channel_id: String,
        added: Vec<MemberPackage>,
        removed: Option<String>,
    },
}
impl Intent {
    fn channel_id(&self) -> &str {
        match self {
            Self::Init { channel_id, .. }
            | Self::Message { channel_id, .. }
            | Self::Members { channel_id, .. } => channel_id,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Pending {
    public: Prepared,
    provider: ProviderSnapshot,
    channel: Channel,
    plaintext: Option<String>,
    intent: Intent,
    rejection_provider: Option<ProviderSnapshot>,
    rejection_channel: Option<Channel>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Vault {
    version: u32,
    pub user_id: i64,
    server_url: String,
    provider: ProviderSnapshot,
    signer_public: Vec<u8>,
    pub channels: BTreeMap<String, Channel>,
    pending: Option<Pending>,
    retry: Option<Intent>,
    pub recovered_drafts: BTreeMap<String, String>,
    pub key_package_hex: Option<String>,
}
impl Vault {
    pub fn fresh(user_id: i64, server_url: String) -> Result<Self, String> {
        let provider = PersistentCryptoProvider::default();
        let (package, signer) =
            key_package::generate_key_package(user_id, &provider).map_err(error)?;
        Ok(Self {
            version: 1,
            user_id,
            server_url,
            provider: provider.snapshot().map_err(error)?,
            signer_public: signer.public().to_vec(),
            channels: BTreeMap::new(),
            pending: None,
            retry: None,
            recovered_drafts: BTreeMap::new(),
            key_package_hex: Some(hex(
                &key_package::serialize_key_package(&package).map_err(error)?
            )),
        })
    }
    fn group_id(&self, channel_id: &str) -> Vec<u8> {
        Sha256::digest(format!("opencorde-mls-v2\0{}\0{}", self.server_url, channel_id).as_bytes())
            .to_vec()
    }
    pub fn request_rejoin(&mut self, channel_id: &str) -> Result<(), String> {
        self.no_pending()?;
        if let Some(channel) = self.channels.get_mut(channel_id) {
            channel.active = false;
        }
        // A consumed enrollment package must never be reoffered for rejoin.
        self.key_package_hex = None;
        self.ensure_key_package()
    }
    pub fn validate(&self, user_id: i64, server_url: &str) -> Result<(), String> {
        if self.version != 1 || self.user_id != user_id || self.server_url != server_url {
            return Err("unsupported or mismatched private vault".into());
        }
        self.provider_and_signer().map(|_| ())
    }
    fn provider_and_signer(&self) -> Result<(PersistentCryptoProvider, SignatureKeyPair), String> {
        let provider = PersistentCryptoProvider::restore(&self.provider).map_err(error)?;
        let signer = SignatureKeyPair::read(
            provider.key_store(),
            &self.signer_public,
            SignatureScheme::ED25519,
        )
        .ok_or("private signer missing from vault")?;
        Ok((provider, signer))
    }
    pub fn ensure_key_package(&mut self) -> Result<(), String> {
        if self.key_package_hex.is_some() {
            return Ok(());
        }
        self.no_pending()?;
        let (provider, signer) = self.provider_and_signer()?;
        let package =
            key_package::generate_key_package_with_signer(self.user_id, &signer, &provider)
                .map_err(error)?;
        self.key_package_hex = Some(hex(
            &key_package::serialize_key_package(&package).map_err(error)?
        ));
        self.provider = provider.snapshot().map_err(error)?;
        Ok(())
    }
    pub fn confirm_key_package(&mut self, package_hex: &str) -> Result<(), String> {
        if self.key_package_hex.as_deref() == Some(package_hex) {
            self.key_package_hex = None;
        }
        Ok(())
    }
    fn no_pending(&self) -> Result<(), String> {
        if self.retry.is_some() {
            return Err(
                "an encrypted operation awaits rebase; synchronize it before preparing another"
                    .into(),
            );
        }
        self.no_inflight()
    }
    fn no_inflight(&self) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("an encrypted operation is pending delivery; synchronize it first".into());
        }
        Ok(())
    }
    pub fn pending(&self) -> Option<Prepared> {
        self.pending.as_ref().map(|pending| pending.public.clone())
    }
    fn operation(&self, channel: &str, kind: &str) -> Operation {
        let state = self.channels.get(channel);
        Operation {
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_sequence: state.map_or(0, |state| state.cursor).to_string(),
            epoch: state.map_or(0, |state| state.epoch).to_string(),
            kind: kind.into(),
            ciphertext: String::new(),
            added: vec![],
            removed: vec![],
            reply_to_id: None,
            attachments: vec![],
            allow_legacy_rejoin: false,
        }
    }
    fn channel(&self, id: &str) -> Result<&Channel, String> {
        let channel = self.channels.get(id).ok_or(
            "no durable group for this channel; ask an administrator to invite this device",
        )?;
        if !channel.active {
            return Err("this device was removed; ask an administrator to invite it again".into());
        }
        Ok(channel)
    }
    fn staged(
        &mut self,
        id: &str,
        operation: Operation,
        group: &MlsGroup,
        provider: &PersistentCryptoProvider,
        plaintext: Option<String>,
        intent: Intent,
    ) -> Result<Prepared, String> {
        let mut channel = self.channels.get(id).cloned().unwrap_or(Channel {
            group: vec![],
            cursor: 0,
            epoch: 0,
            active: true,
            cache: VecDeque::new(),
            file_keys: BTreeMap::new(),
            welcome_ack: None,
        });
        channel.group = group::serialize_group(group).map_err(error)?;
        channel.epoch = group::epoch(group);
        channel.active = group::member_ids(group)
            .map_err(error)?
            .contains(&self.user_id);
        if channel.active {
            channel.file_keys.insert(
                channel.epoch,
                group::export_file_key(group, provider).map_err(error)?,
            );
            while channel.file_keys.len() > 32 {
                channel.file_keys.pop_first();
            }
        }
        let public = Prepared {
            channel_id: id.into(),
            operation,
        };
        let snapshot = provider.snapshot().map_err(error)?;
        let rejection_provider = (public.operation.kind == "application").then(|| snapshot.clone());
        let rejection_channel = (public.operation.kind == "application").then(|| channel.clone());
        if public.operation.kind == "application" {
            self.recovered_drafts.remove(id);
        }
        self.pending = Some(Pending {
            public: public.clone(),
            provider: snapshot,
            channel,
            plaintext,
            intent,
            rejection_provider,
            rejection_channel,
        });
        Ok(public)
    }
    pub fn prepare_init(&mut self, id: &str, legacy: bool) -> Result<Prepared, String> {
        self.no_pending()?;
        if self.channels.contains_key(id) {
            return Err("a durable group already exists; it cannot be silently replaced".into());
        }
        let (provider, signer) = self.provider_and_signer()?;
        let credential =
            Credential::new(self.user_id.to_le_bytes().to_vec(), CredentialType::Basic)
                .map_err(error)?;
        let group_id = self.group_id(id);
        let group = group::create_group_with_id(
            CredentialWithKey {
                credential,
                signature_key: signer.public().into(),
            },
            &signer,
            &provider,
            &group_id,
        )
        .map_err(error)?;
        let mut operation = self.operation(id, "init");
        operation.allow_legacy_rejoin = legacy;
        self.staged(
            id,
            operation,
            &group,
            &provider,
            None,
            Intent::Init {
                channel_id: id.into(),
                legacy,
            },
        )
    }
    pub fn prepare_message(
        &mut self,
        id: &str,
        plaintext: String,
        reply_to_id: Option<String>,
        attachments: Vec<serde_json::Value>,
    ) -> Result<Prepared, String> {
        self.no_pending()?;
        if attachments.len() > 10
            || serde_json::to_vec(&attachments).map_err(error)?.len() > 64 * 1024
        {
            return Err("encrypted attachment metadata exceeds limit".into());
        }
        if let Some(reply) = &reply_to_id {
            if number(reply)? == 0 {
                return Err("invalid reply identity".into());
            }
        }
        if plaintext.len() > 64 * 1024 {
            return Err("message exceeds encrypted local limit".into());
        }
        let intent = Intent::Message {
            channel_id: id.into(),
            plaintext: plaintext.clone(),
            reply_to_id: reply_to_id.clone(),
            attachments: attachments.clone(),
        };
        let channel = self.channel(id)?;
        for attachment in &attachments {
            if attachment
                .get("encryption_epoch")
                .and_then(|value| value.as_str())
                != Some(&channel.epoch.to_string())
            {
                return Err(
                    "membership changed after file encryption; retain the draft and reattach files"
                        .into(),
                );
            }
        }
        let mut group = group::deserialize_group(&channel.group).map_err(error)?;
        let (provider, signer) = self.provider_and_signer()?;
        let bytes = encrypt::encrypt_message(&mut group, plaintext.as_bytes(), &signer, &provider)
            .map_err(error)?;
        let mut operation = self.operation(id, "application");
        operation.ciphertext = URL_SAFE_NO_PAD.encode(bytes);
        operation.reply_to_id = reply_to_id;
        operation.attachments = attachments
            .into_iter()
            .map(|mut attachment| {
                if let Some(object) = attachment.as_object_mut() {
                    object.remove("encryption_epoch");
                }
                attachment
            })
            .collect();
        self.staged(id, operation, &group, &provider, Some(plaintext), intent)
    }
    pub fn prepare_members(
        &mut self,
        id: &str,
        added: Vec<MemberPackage>,
        removed: Option<String>,
    ) -> Result<Prepared, String> {
        self.no_pending()?;
        let mut group = group::deserialize_group(&self.channel(id)?.group).map_err(error)?;
        let (provider, signer) = self.provider_and_signer()?;
        let mut operation = self.operation(id, "commit");
        let intent = Intent::Members {
            channel_id: id.into(),
            added: added.clone(),
            removed: removed.clone(),
        };
        let bytes = if let Some(removed) = removed {
            if !added.is_empty() {
                return Err("add and remove must be separate commits".into());
            }
            let user = removed
                .parse::<i64>()
                .map_err(|_| "invalid member identity")?;
            let bytes =
                group::stage_remove_member(&mut group, user, &signer, &provider).map_err(error)?;
            operation.removed.push(removed);
            bytes
        } else {
            if added.is_empty() {
                return Err("no members to add".into());
            }
            let mut packages = Vec::new();
            let mut ids = std::collections::BTreeSet::new();
            for member in &added {
                let package = key_package::deserialize_key_package(
                    &unhex(&member.key_package_hex)?,
                    &provider,
                )
                .map_err(error)?;
                let expected = member
                    .user_id
                    .parse::<i64>()
                    .map_err(|_| "invalid member identity")?;
                if expected <= 0
                    || package.leaf_node().credential().identity() != expected.to_le_bytes()
                    || !ids.insert(expected)
                {
                    return Err("key package identity does not match the invited member".into());
                }
                packages.push(package);
            }
            let (commit, welcome) =
                group::stage_add_members(&mut group, &packages, &signer, &provider)
                    .map_err(error)?;
            operation.added = added
                .iter()
                .map(|member| WelcomeTarget {
                    user_id: member.user_id.clone(),
                    welcome: URL_SAFE_NO_PAD.encode(&welcome),
                })
                .collect();
            commit
        };
        operation.ciphertext = URL_SAFE_NO_PAD.encode(bytes);
        // Preserve consumed outgoing handshake ratchets even if the server CAS
        // rejects this commit. Only the pending epoch transition is discarded.
        let rejection_provider = provider.snapshot().map_err(error)?;
        let mut rejected_group =
            group::deserialize_group(&group::serialize_group(&group).map_err(error)?)
                .map_err(error)?;
        rejected_group.clear_pending_commit();
        let mut rejection_channel = self.channel(id)?.clone();
        rejection_channel.group = group::serialize_group(&rejected_group).map_err(error)?;
        group.merge_pending_commit(&provider).map_err(error)?;
        let public = self.staged(id, operation, &group, &provider, None, intent)?;
        let pending = self.pending.as_mut().ok_or("missing staged commit")?;
        pending.rejection_provider = Some(rejection_provider);
        pending.rejection_channel = Some(rejection_channel);
        Ok(public)
    }
    pub fn reject_conflict(&mut self, operation_id: &str) -> Result<(), String> {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.public.operation.operation_id == operation_id)
        {
            let pending = self.pending.take().ok_or("missing staged operation")?;
            if let Some(provider) = pending.rejection_provider {
                self.provider = provider;
            }
            if let Some(channel) = pending.rejection_channel {
                self.channels.insert(pending.public.channel_id, channel);
            }
            self.retry = Some(pending.intent);
            return Ok(());
        }
        Err("conflict did not identify the pending operation".into())
    }
    pub fn cancel_retry(&mut self, channel_id: &str) -> Result<(), String> {
        let intent = self
            .retry
            .as_ref()
            .ok_or("there is no definitively rejected operation to cancel")?;
        if intent.channel_id() != channel_id {
            return Err("retry belongs to another channel".into());
        }
        if let Intent::Message { plaintext, .. } = intent {
            self.recovered_drafts
                .insert(channel_id.into(), plaintext.clone());
        }
        self.retry = None;
        Ok(())
    }
    pub fn retry_channel(&self) -> Option<String> {
        self.retry
            .as_ref()
            .map(|intent| intent.channel_id().to_owned())
    }
    pub fn retry_operation(&mut self) -> Result<Prepared, String> {
        let intent = self
            .retry
            .take()
            .ok_or("no encrypted operation awaits retry")?;
        match intent {
            Intent::Init { channel_id, legacy } => self.prepare_init(&channel_id, legacy),
            Intent::Message {
                channel_id,
                plaintext,
                reply_to_id,
                attachments,
            } => self.prepare_message(&channel_id, plaintext, reply_to_id, attachments),
            Intent::Members {
                channel_id,
                added,
                removed,
            } => self.prepare_members(&channel_id, added, removed),
        }
    }
    pub fn accept(&mut self, event: &Event) -> Result<(), String> {
        let pending = self
            .pending
            .as_ref()
            .ok_or("no operation awaits acceptance")?;
        let request = &pending.public.operation;
        let mut added: Vec<_> = request
            .added
            .iter()
            .map(|member| member.user_id.clone())
            .collect();
        let mut removed = request.removed.clone();
        let mut event_added = event.added.clone();
        let mut event_removed = event.removed.clone();
        added.sort();
        removed.sort();
        event_added.sort();
        event_removed.sort();
        if event.operation_id != request.operation_id
            || event.kind != request.kind
            || event.ciphertext != request.ciphertext
            || event.sender_id != self.user_id.to_string()
            || number(&event.sequence)? != number(&request.expected_sequence)? + 1
            || number(&event.epoch)? != pending.channel.epoch
            || added != event_added
            || removed != event_removed
        {
            return Err("relay acceptance does not match the staged encrypted operation".into());
        }
        let mut channel = pending.channel.clone();
        channel.cursor = number(&event.sequence)?;
        if let Some(plaintext) = &pending.plaintext {
            cache(&mut channel, event, plaintext.clone())?;
        }
        self.channels
            .insert(pending.public.channel_id.clone(), channel);
        self.provider = pending.provider.clone();
        self.pending = None;
        self.trim_cache();
        Ok(())
    }
    pub fn apply(&mut self, id: &str, event: &Event) -> Result<(), String> {
        self.no_inflight()?;
        let mut channel = self.channel(id)?.clone();
        let sequence = number(&event.sequence)?;
        if sequence <= channel.cursor {
            return Ok(());
        }
        if sequence != channel.cursor + 1 {
            return Err("encrypted event sequence has a gap; fetch missing events".into());
        }
        let provider = PersistentCryptoProvider::restore(&self.provider).map_err(error)?;
        let mut group = group::deserialize_group(&channel.group).map_err(error)?;
        let before = group::member_ids(&group).map_err(error)?;
        let (sender, plaintext) =
            encrypt::decrypt_authenticated(&mut group, &decode(&event.ciphertext)?, &provider)
                .map_err(error)?;
        if sender.to_string() != event.sender_id || group::epoch(&group) != number(&event.epoch)? {
            return Err("relay metadata differs from authenticated MLS sender or epoch".into());
        }
        let after = group::member_ids(&group).map_err(error)?;
        let expected_added: Vec<String> = after
            .iter()
            .filter(|id| !before.contains(id))
            .map(ToString::to_string)
            .collect();
        let expected_removed: Vec<String> = before
            .iter()
            .filter(|id| !after.contains(id))
            .map(ToString::to_string)
            .collect();
        let mut added = event.added.clone();
        let mut removed = event.removed.clone();
        added.sort();
        removed.sort();
        let mut expected_added = expected_added;
        let mut expected_removed = expected_removed;
        expected_added.sort();
        expected_removed.sort();
        if added != expected_added || removed != expected_removed {
            return Err("relay membership differs from authenticated MLS commit".into());
        }
        match (&*event.kind, plaintext) {
            ("application", Some(bytes)) => {
                if channel.epoch != group::epoch(&group) {
                    return Err("application changed MLS epoch".into());
                }
                cache(
                    &mut channel,
                    event,
                    String::from_utf8(bytes).map_err(|_| "invalid UTF-8 message")?,
                )?;
            }
            ("commit", None) if group::epoch(&group) == channel.epoch + 1 => (),
            _ => return Err("relay kind differs from authenticated MLS content".into()),
        }
        channel.cursor = sequence;
        channel.epoch = group::epoch(&group);
        channel.active = after.contains(&self.user_id);
        channel.group = group::serialize_group(&group).map_err(error)?;
        if channel.active {
            channel.file_keys.insert(
                channel.epoch,
                group::export_file_key(&group, &provider).map_err(error)?,
            );
            while channel.file_keys.len() > 32 {
                channel.file_keys.pop_first();
            }
        }
        self.provider = provider.snapshot().map_err(error)?;
        self.channels.insert(id.into(), channel);
        self.trim_cache();
        Ok(())
    }
    pub fn join(&mut self, id: &str, welcome: &Welcome) -> Result<(), String> {
        self.no_pending()?;
        if let Some(channel) = self.channels.get(id) {
            if channel.welcome_ack.as_deref() == Some(&welcome.operation_id) {
                return Ok(());
            }
            if channel.active {
                return Err("this channel is already bound to a different group; explicit rejoin is required".into());
            }
        }
        let provider = PersistentCryptoProvider::restore(&self.provider).map_err(error)?;
        let group = group::process_welcome(&decode(&welcome.welcome)?, &provider).map_err(error)?;
        if group.group_id().as_slice() != self.group_id(id).as_slice() {
            return Err("Welcome belongs to another server or channel".into());
        }
        if group::epoch(&group) != number(&welcome.epoch)?
            || !group::member_ids(&group)
                .map_err(error)?
                .contains(&self.user_id)
        {
            return Err("Welcome does not match this account or epoch".into());
        }
        let mut file_keys = BTreeMap::new();
        file_keys.insert(
            group::epoch(&group),
            group::export_file_key(&group, &provider).map_err(error)?,
        );
        let channel = Channel {
            group: group::serialize_group(&group).map_err(error)?,
            cursor: number(&welcome.sequence)?,
            epoch: group::epoch(&group),
            active: true,
            cache: VecDeque::new(),
            file_keys,
            welcome_ack: Some(welcome.operation_id.clone()),
        };
        self.channels.insert(id.into(), channel);
        self.provider = provider.snapshot().map_err(error)?;
        Ok(())
    }
    pub fn ack_welcome(&mut self, id: &str, operation: &str) -> Result<(), String> {
        let channel = self.channels.get_mut(id).ok_or("channel is not joined")?;
        if channel.welcome_ack.as_deref() == Some(operation) {
            channel.welcome_ack = None;
        }
        Ok(())
    }
    pub fn message(
        &self,
        id: &str,
        message_id: &str,
        ciphertext_hex: &str,
    ) -> Result<Option<String>, String> {
        let Some(channel) = self.channels.get(id) else {
            return Ok(None);
        };
        let digest = hash(&unhex(ciphertext_hex)?);
        Ok(channel
            .cache
            .iter()
            .find(|entry| entry.id == message_id && entry.digest == digest)
            .map(|entry| entry.plaintext.clone()))
    }
    pub fn voice_key(&self, id: &str) -> Result<Vec<u8>, String> {
        let channel = self.channel(id)?;
        let provider = PersistentCryptoProvider::restore(&self.provider).map_err(error)?;
        group::export_voice_key(
            &group::deserialize_group(&channel.group).map_err(error)?,
            &provider,
        )
        .map_err(error)
    }
    pub fn file_key(&self, id: &str, epoch: Option<u64>) -> Result<(u64, [u8; 32]), String> {
        let channel = self.channels.get(id).ok_or("no durable group for file")?;
        let epoch = epoch.unwrap_or(channel.epoch);
        let key = channel
            .file_keys
            .get(&epoch)
            .ok_or("file epoch is outside retained local history")?;
        Ok((
            epoch,
            key.clone().try_into().map_err(|_| "invalid file key")?,
        ))
    }
    fn trim_cache(&mut self) {
        for channel in self.channels.values_mut() {
            while channel.cache.len() > CACHE_PER_CHANNEL {
                channel.cache.pop_front();
            }
        }
        while self
            .channels
            .values()
            .flat_map(|channel| &channel.cache)
            .map(|entry| entry.plaintext.len())
            .sum::<usize>()
            > CACHE_BYTES
        {
            if let Some(channel) = self
                .channels
                .values_mut()
                .find(|channel| !channel.cache.is_empty())
            {
                channel.cache.pop_front();
            } else {
                break;
            }
        }
    }
}
fn cache(channel: &mut Channel, event: &Event, plaintext: String) -> Result<(), String> {
    let id = event
        .message_id
        .clone()
        .ok_or("application event has no message id")?;
    channel.cache.push_back(Cached {
        id,
        digest: hash(&decode(&event.ciphertext)?),
        plaintext,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn accepted(prepared: &Prepared) -> Event {
        let op = &prepared.operation;
        serde_json::from_value(serde_json::json!({
            "operation_id": op.operation_id, "sequence": (number(&op.expected_sequence).unwrap()+1).to_string(),
            "epoch": (number(&op.epoch).unwrap()+u64::from(op.kind == "commit")).to_string(),
            "sender_id":"1", "kind":op.kind, "ciphertext":op.ciphertext,
            "added":op.added.iter().map(|v|v.user_id.clone()).collect::<Vec<_>>(), "removed":op.removed,
            "message_id":null
        })).unwrap()
    }
    #[test]
    fn relay_bounds_are_canonical_and_bounded() {
        for bad in [
            "",
            "-1",
            "+1",
            "01",
            " 1",
            "9223372036854775808",
            "18446744073709551615",
        ] {
            assert!(number(bad).is_err(), "accepted {bad}");
        }
        assert_eq!(number("9223372036854775807").unwrap(), i64::MAX as u64);
        assert_eq!(number("0").unwrap(), 0);
        assert!(decode(&"A".repeat(262145)).is_err());
    }
    #[test]
    fn rejected_operations_preserve_consumed_ratchets_and_allow_cancel() {
        let mut vault = Vault::fresh(1, "https://example.test/api/v1".into()).unwrap();
        let init = vault.prepare_init("9", false).unwrap();
        vault.accept(&accepted(&init)).unwrap();
        assert!(
            vault
                .prepare_message("9", "draft".into(), None, vec![serde_json::json!({}); 11])
                .is_err()
        );
        assert!(vault.pending.is_none());
        let before = vault.channels["9"].group.clone();
        let prepared = vault
            .prepare_message("9", "draft".into(), None, vec![])
            .unwrap();
        let consumed = vault.pending.as_ref().unwrap().channel.group.clone();
        assert_ne!(before, consumed);
        vault
            .reject_conflict(&prepared.operation.operation_id)
            .unwrap();
        assert_eq!(vault.channels["9"].group, consumed);
        vault.cancel_retry("9").unwrap();
        assert_eq!(vault.recovered_drafts["9"], "draft");
        let bob = Vault::fresh(2, "https://example.test/api/v1".into()).unwrap();
        let member = MemberPackage {
            user_id: "2".into(),
            key_package_hex: bob.key_package_hex.unwrap(),
        };
        let prepared = vault.prepare_members("9", vec![member], None).unwrap();
        let expected = vault
            .pending
            .as_ref()
            .unwrap()
            .rejection_channel
            .as_ref()
            .unwrap()
            .group
            .clone();
        vault
            .reject_conflict(&prepared.operation.operation_id)
            .unwrap();
        assert_eq!(vault.channels["9"].group, expected);
        assert_eq!(vault.channels["9"].epoch, 0);
        assert_eq!(vault.recovered_drafts["9"], "draft");
        vault.cancel_retry("9").unwrap();
        assert!(vault.retry.is_none());
    }
}
