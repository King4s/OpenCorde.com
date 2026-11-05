//! File envelopes carry an authenticated MLS epoch lookup. Old epoch keys are
//! retained in the encrypted vault for at most 32 membership epochs.
use crate::commands::crypto::CryptoState;
use base64::{Engine, engine::general_purpose::STANDARD};
use opencorde_crypto::file_crypto;
use tauri::State;
const HEADER: &[u8] = b"OCFILE2";

#[tauri::command(rename_all = "snake_case")]
pub fn crypto_encrypt_file(
    session_id: String,
    channel_id: String,
    file_data_b64: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    state.with_session(&session_id, |store| {
        let data = store.data()?;
        if !data
            .channels
            .get(&channel_id)
            .is_some_and(|channel| channel.active)
        {
            return Err("encrypted channel is not active".into());
        }
        let (epoch, key) = data.file_key(&channel_id, None)?;
        let plaintext = STANDARD
            .decode(file_data_b64)
            .map_err(|_| "invalid file encoding")?;
        let encrypted =
            file_crypto::encrypt_bytes(&key, &plaintext).map_err(|error| error.to_string())?;
        let mut envelope = HEADER.to_vec();
        envelope.extend(epoch.to_be_bytes());
        envelope.extend(encrypted);
        Ok(STANDARD.encode(envelope))
    })
}
#[tauri::command(rename_all = "snake_case")]
pub fn crypto_decrypt_file(
    session_id: String,
    channel_id: String,
    encrypted_b64: String,
    state: State<'_, CryptoState>,
) -> Result<String, String> {
    state.with_session(&session_id, |store| {
        let bytes = STANDARD
            .decode(encrypted_b64)
            .map_err(|_| "invalid file encoding")?;
        let (epoch, encrypted) = if bytes.starts_with(HEADER) && bytes.len() >= HEADER.len() + 8 {
            let epoch = u64::from_be_bytes(
                bytes[HEADER.len()..HEADER.len() + 8]
                    .try_into()
                    .map_err(|_| "invalid file epoch")?,
            );
            (Some(epoch), &bytes[HEADER.len() + 8..])
        } else {
            (None, bytes.as_slice())
        };
        let (_, key) = store.data()?.file_key(&channel_id, epoch)?;
        let plain =
            file_crypto::decrypt_bytes(&key, encrypted).map_err(|error| error.to_string())?;
        Ok(STANDARD.encode(plain))
    })
}
