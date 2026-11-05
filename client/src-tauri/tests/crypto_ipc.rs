//! Execute production IPC handlers in newly spawned processes for every step.
#[path = "../src/commands/crypto.rs"]
pub mod crypto;
mod commands {
    pub use super::crypto;
}
#[path = "../src/commands/file_crypto.rs"]
mod file_crypto;
use opencorde_crypto::durable_store::{Account, KeyMaterial, KeyProvider};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};
use tauri::{App, WebviewWindow, test::MockRuntime};

struct TestKeys(PathBuf);
impl TestKeys {
    fn path(&self, account: &Account) -> PathBuf {
        use sha2::{Digest, Sha256};
        self.0.with_file_name(format!(
            "test-key-{}.json",
            crypto::vault::hex(&Sha256::digest(serde_json::to_vec(account).unwrap()))
        ))
    }
}
impl KeyProvider for TestKeys {
    fn load(&self, account: &Account) -> Result<Option<KeyMaterial>, String> {
        match std::fs::read(self.path(account)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn save(&self, account: &Account, key: &KeyMaterial) -> Result<(), String> {
        std::fs::write(self.path(account), serde_json::to_vec(key).unwrap())
            .map_err(|error| error.to_string())
    }
}
fn client(directory: &Path) -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let state = if std::env::var_os("OPENCORDE_TEST_OS_KEYS").is_some() {
        crypto::CryptoState::isolated_os_keys(directory.join("vault"))
    } else {
        crypto::CryptoState::isolated(
            directory.join("vault"),
            Arc::new(TestKeys(directory.join("test-key.json"))),
        )
    };
    let app = tauri::test::mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            crypto::crypto_init,
            crypto::crypto_lock,
            crypto::crypto_status,
            crypto::crypto_key_package,
            crypto::crypto_prepare_init,
            crypto::crypto_prepare_message,
            crypto::crypto_prepare_members,
            crypto::crypto_accept_event,
            crypto::crypto_reject_conflict,
            crypto::crypto_retry_operation,
            crypto::crypto_cancel_retry,
            crypto::crypto_apply_events,
            crypto::crypto_join,
            crypto::crypto_request_rejoin,
            crypto::crypto_ack_welcome,
            crypto::crypto_cached_message,
            crypto::crypto_export_voice_key,
            file_crypto::crypto_encrypt_file,
            file_crypto::crypto_decrypt_file,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    (app, window)
}
fn invoke(window: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}
fn directory() -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("opencorde-ipc-restart-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    directory
}
fn attempt(directory: &Path, user: &str, command: &str, args: Value) -> Value {
    std::fs::create_dir_all(directory).unwrap();
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&json!({
            "user":user,"command":command,"args":args
        }))
        .unwrap(),
    )
    .unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "ipc_process_worker", "--nocapture"])
        .env("OPENCORDE_IPC_TEST_DIRECTORY", directory)
        .status()
        .unwrap();
    assert!(status.success(), "process failed for {command}");
    let response: Value =
        serde_json::from_slice(&std::fs::read(directory.join("response.json")).unwrap()).unwrap();
    response
}
fn step(directory: &Path, user: &str, command: &str, args: Value) -> Value {
    let response = attempt(directory, user, command, args);
    assert!(response.get("error").is_none(), "IPC {command}: {response}");
    response["result"].clone()
}
#[test]
fn ipc_process_worker() {
    let Some(path) = std::env::var_os("OPENCORDE_IPC_TEST_DIRECTORY") else {
        return;
    };
    let directory = Path::new(&path);
    let request: Value =
        serde_json::from_slice(&std::fs::read(directory.join("request.json")).unwrap()).unwrap();
    let (_app, window) = client(directory);
    let session = invoke(&window, "crypto_init", json!({
        "server_url":request.get("server_url").and_then(Value::as_str).unwrap_or("https://restart.example.test"), "user_id":request["user"],
    })).unwrap();
    let mut args = request["args"].clone();
    args["session_id"] = session["session_id"].clone();
    let result = invoke(&window, request["command"].as_str().unwrap(), args);
    let response = match result {
        Ok(result) => json!({"result":result}),
        Err(error) => json!({"error":error}),
    };
    std::fs::write(
        directory.join("response.json"),
        serde_json::to_vec(&response).unwrap(),
    )
    .unwrap();
}
fn event(prepared: &Value, sender: &str) -> Value {
    let operation = &prepared["operation"];
    let epoch: u64 = operation["epoch"].as_str().unwrap().parse().unwrap();
    let sequence: u64 = operation["expected_sequence"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    json!({
        "operation_id":operation["operation_id"],"sequence":(sequence+1).to_string(),
        "epoch":(epoch+u64::from(operation["kind"]=="commit")).to_string(),
        "sender_id":sender,"kind":operation["kind"],"ciphertext":operation["ciphertext"],
        "added":operation["added"].as_array().unwrap().iter().map(|member| member["user_id"].clone()).collect::<Vec<_>>(),
        "removed":operation["removed"],"message_id":if operation["kind"]=="application" {Some(format!("message-{}",sequence+1))} else {None},
    })
}
#[test]
fn real_process_restarts_recover_identity_welcome_ratchets_and_outbox() {
    let root = directory();
    let alice = root.join("alice");
    let bob = root.join("bob");
    let aid = "9007199254740993";
    let bid = "9007199254740995";
    let init = step(
        &alice,
        aid,
        "crypto_prepare_init",
        json!({"channel_id":"channel","allow_legacy_rejoin":false}),
    );
    let after_restart = step(&alice, aid, "crypto_status", json!({}));
    assert_eq!(
        after_restart["pending"], init,
        "prepared init must survive process death"
    );
    step(
        &alice,
        aid,
        "crypto_accept_event",
        json!({"event":event(&init,aid)}),
    );
    let bob_status = step(&bob, bid, "crypto_status", json!({}));
    let added = step(
        &alice,
        aid,
        "crypto_prepare_members",
        json!({
            "channel_id":"channel","added":[{"user_id":bid,"key_package_hex":bob_status["key_package_hex"]}],"removed":null
        }),
    );
    let added_event = event(&added, aid);
    step(
        &alice,
        aid,
        "crypto_accept_event",
        json!({"event":added_event}),
    );
    let welcome = json!({"operation_id":added_event["operation_id"],"sequence":added_event["sequence"],
        "epoch":added_event["epoch"],"welcome":added["operation"]["added"][0]["welcome"]});
    assert!(
        attempt(
            &bob,
            bid,
            "crypto_join",
            json!({"channel_id":"swapped-channel","welcome":welcome})
        )
        .get("error")
        .is_some(),
        "valid Welcome must reject a different channel binding"
    );
    step(
        &bob,
        bid,
        "crypto_join",
        json!({"channel_id":"channel","welcome":welcome}),
    );
    let bob_restarted = step(&bob, bid, "crypto_status", json!({}));
    assert_eq!(
        bob_restarted["channels"][0]["welcome_ack"],
        added_event["operation_id"]
    );
    // Retrying the same unacknowledged Welcome after restart is harmless.
    step(
        &bob,
        bid,
        "crypto_join",
        json!({"channel_id":"channel","welcome":welcome}),
    );
    step(
        &bob,
        bid,
        "crypto_ack_welcome",
        json!({"channel_id":"channel","operation_id":added_event["operation_id"]}),
    );
    let message = step(
        &alice,
        aid,
        "crypto_prepare_message",
        json!({
            "channel_id":"channel","plaintext":"persisted native hello","reply_to_id":null,"attachments":[]
        }),
    );
    assert_eq!(
        step(&alice, aid, "crypto_status", json!({}))["pending"],
        message
    );
    let message_event = event(&message, aid);
    step(
        &alice,
        aid,
        "crypto_accept_event",
        json!({"event":message_event}),
    );
    let unchanged = step(&bob, bid, "crypto_status", json!({}))["channels"].clone();
    for (field, value) in [
        ("sender_id", json!(bid)),
        ("added", json!([bid])),
        ("sequence", json!("999")),
    ] {
        let mut forged = message_event.clone();
        forged[field] = value;
        assert!(
            attempt(
                &bob,
                bid,
                "crypto_apply_events",
                json!({"channel_id":"channel","events":[forged]})
            )
            .get("error")
            .is_some(),
            "forged {field} must reject"
        );
        assert_eq!(
            step(&bob, bid, "crypto_status", json!({}))["channels"],
            unchanged,
            "rejection must not consume durable receive state"
        );
    }
    step(
        &bob,
        bid,
        "crypto_apply_events",
        json!({"channel_id":"channel","events":[message_event]}),
    );
    let ciphertext = crypto::vault::hex(
        &base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            message_event["ciphertext"].as_str().unwrap(),
        )
        .unwrap(),
    );
    assert_eq!(
        step(
            &bob,
            bid,
            "crypto_cached_message",
            json!({
                "channel_id":"channel","message_id":message_event["message_id"],"ciphertext_hex":ciphertext
            })
        ),
        "persisted native hello"
    );
    // Replaying history returns the cached display without consuming ratchets twice.
    step(
        &bob,
        bid,
        "crypto_apply_events",
        json!({"channel_id":"channel","events":[message_event]}),
    );
    let reply = step(
        &bob,
        bid,
        "crypto_prepare_message",
        json!({
            "channel_id":"channel","plaintext":"persisted reply","reply_to_id":null,"attachments":[]
        }),
    );
    let reply_event = event(&reply, bid);
    step(
        &bob,
        bid,
        "crypto_accept_event",
        json!({"event":reply_event}),
    );
    step(
        &alice,
        aid,
        "crypto_apply_events",
        json!({"channel_id":"channel","events":[reply_event]}),
    );
    assert_eq!(
        step(
            &alice,
            aid,
            "crypto_export_voice_key",
            json!({"channel_id":"channel"})
        ),
        step(
            &bob,
            bid,
            "crypto_export_voice_key",
            json!({"channel_id":"channel"})
        )
    );
    let file = step(
        &alice,
        aid,
        "crypto_encrypt_file",
        json!({"channel_id":"channel","file_data_b64":"aGVsbG8="}),
    );
    assert_eq!(
        step(
            &bob,
            bid,
            "crypto_decrypt_file",
            json!({"channel_id":"channel","encrypted_b64":file})
        ),
        "aGVsbG8="
    );
    // Bob never sees the removal event because server membership denies access.
    let removed = step(
        &alice,
        aid,
        "crypto_prepare_members",
        json!({"channel_id":"channel","added":[],"removed":bid}),
    );
    step(
        &alice,
        aid,
        "crypto_accept_event",
        json!({"event":event(&removed,aid)}),
    );
    let enrollment = step(
        &bob,
        bid,
        "crypto_request_rejoin",
        json!({"channel_id":"channel"}),
    );
    assert!(!enrollment["channels"][0]["active"].as_bool().unwrap());
    let readded = step(
        &alice,
        aid,
        "crypto_prepare_members",
        json!({
            "channel_id":"channel","added":[{"user_id":bid,"key_package_hex":enrollment["key_package_hex"]}],"removed":null
        }),
    );
    let readded_event = event(&readded, aid);
    step(
        &alice,
        aid,
        "crypto_accept_event",
        json!({"event":readded_event}),
    );
    step(
        &bob,
        bid,
        "crypto_join",
        json!({"channel_id":"channel","welcome":{
            "operation_id":readded_event["operation_id"],"sequence":readded_event["sequence"],"epoch":readded_event["epoch"],
            "welcome":readded["operation"]["added"][0]["welcome"]
        }}),
    );
    assert_eq!(
        step(
            &alice,
            aid,
            "crypto_export_voice_key",
            json!({"channel_id":"channel"})
        ),
        step(
            &bob,
            bid,
            "crypto_export_voice_key",
            json!({"channel_id":"channel"})
        )
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stale_session_and_invalid_snowflake_cannot_access_another_account() {
    let root = directory();
    let (_app, window) = client(&root);
    assert!(
        invoke(
            &window,
            "crypto_init",
            json!({"server_url":"https://example.test","user_id":9007199254740993_i64})
        )
        .is_err()
    );
    let alice = invoke(
        &window,
        "crypto_init",
        json!({"server_url":"https://example.test/a","user_id":"9007199254740993"}),
    )
    .unwrap();
    let bob = invoke(
        &window,
        "crypto_init",
        json!({"server_url":"https://example.test/b","user_id":"9007199254740995"}),
    )
    .unwrap();
    assert_ne!(alice["session_id"], bob["session_id"]);
    assert!(
        invoke(
            &window,
            "crypto_status",
            json!({"session_id":alice["session_id"]})
        )
        .is_err()
    );
    invoke(
        &window,
        "crypto_lock",
        json!({"session_id":bob["session_id"]}),
    )
    .unwrap();
    assert!(
        invoke(
            &window,
            "crypto_status",
            json!({"session_id":bob["session_id"]})
        )
        .is_err()
    );
    drop(window);
    drop(_app);
    std::fs::remove_dir_all(root).unwrap();
}

/// Test-only persistent IPC bridge used by the rendered frontend acceptance test.
/// Arguments and session IDs are passed unchanged to generated production wrappers.
#[test]
fn ipc_stream_worker() {
    use std::io::{BufRead, Write};
    let Some(path) = std::env::var_os("OPENCORDE_IPC_STREAM_DIRECTORY") else {
        return;
    };
    let directory = Path::new(&path);
    std::fs::create_dir_all(directory).unwrap();
    let (_app, window) = client(directory);
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        let result = invoke(
            &window,
            request["command"].as_str().unwrap(),
            request["args"].clone(),
        );
        let response = match result {
            Ok(result) => json!({"result":result}),
            Err(error) => json!({"error":error}),
        };
        println!("IPC_RESPONSE:{}", serde_json::to_string(&response).unwrap());
        std::io::stdout().flush().unwrap();
    }
}
