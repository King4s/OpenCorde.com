//! Snapshot-capable OpenMLS 0.5 provider. Snapshots contain private keys and
//! may only be persisted inside the authenticated native encrypted vault.
use openmls_rust_crypto::RustCrypto;
use openmls_traits::{key_store::{MlsEntity, OpenMlsKeyStore}, OpenMlsCryptoProvider};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::RwLock};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProviderSnapshot(pub Vec<(Vec<u8>, Vec<u8>)>);

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum StoreError {
    #[error("private key store lock poisoned")]
    Poisoned,
    #[error("invalid private key store serialization")]
    Serialization,
    #[error("duplicate private key store entry")]
    Duplicate,
}

#[derive(Default)]
pub struct SnapshotKeyStore {
    values: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
}

impl OpenMlsKeyStore for SnapshotKeyStore {
    type Error = StoreError;
    fn store<V: MlsEntity>(&self, key: &[u8], value: &V) -> Result<(), Self::Error> {
        let bytes = serde_json::to_vec(value).map_err(|_| StoreError::Serialization)?;
        self.values.write().map_err(|_| StoreError::Poisoned)?.insert(key.to_vec(), bytes);
        Ok(())
    }
    fn read<V: MlsEntity>(&self, key: &[u8]) -> Option<V> {
        let values = self.values.read().ok()?;
        serde_json::from_slice(values.get(key)?).ok()
    }
    fn delete<V: MlsEntity>(&self, key: &[u8]) -> Result<(), Self::Error> {
        self.values.write().map_err(|_| StoreError::Poisoned)?.remove(key);
        Ok(())
    }
}

#[derive(Default)]
pub struct PersistentCryptoProvider {
    crypto: RustCrypto,
    keys: SnapshotKeyStore,
}

impl PersistentCryptoProvider {
    pub fn snapshot(&self) -> Result<ProviderSnapshot, StoreError> {
        Ok(ProviderSnapshot(self.keys.values.read().map_err(|_| StoreError::Poisoned)?
            .iter().map(|(key, value)| (key.clone(), value.clone())).collect()))
    }
    pub fn restore(snapshot: &ProviderSnapshot) -> Result<Self, StoreError> {
        let mut values = BTreeMap::new();
        for (key, value) in &snapshot.0 {
            if values.insert(key.clone(), value.clone()).is_some() {
                return Err(StoreError::Duplicate);
            }
        }
        Ok(Self { crypto: RustCrypto::default(),
            keys: SnapshotKeyStore { values: RwLock::new(values) } })
    }
}

impl OpenMlsCryptoProvider for PersistentCryptoProvider {
    type CryptoProvider = RustCrypto;
    type RandProvider = RustCrypto;
    type KeyStoreProvider = SnapshotKeyStore;
    fn crypto(&self) -> &RustCrypto { &self.crypto }
    fn rand(&self) -> &RustCrypto { &self.crypto }
    fn key_store(&self) -> &SnapshotKeyStore { &self.keys }
}
