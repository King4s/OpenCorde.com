//! Call the production command wrappers through Tauri IPC, not duplicate crypto code.
#[path = "../src/commands/crypto.rs"]
pub mod crypto;
// Match the native application module path used by file_crypto.
mod commands {
    pub use super::crypto;
}
#[path = "../src/commands/file_crypto.rs"]
mod file_crypto;

use serde_json::{json, Value};
use tauri::{test::MockRuntime, App, Manager, WebviewWindow};

fn client() -> (App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = tauri::test::mock_builder()
        .manage(crypto::CryptoState::default())
        .invoke_handler(tauri::generate_handler![
            crypto::crypto_init,
            crypto::crypto_create_group,
            crypto::crypto_add_member,
            crypto::crypto_process_welcome,
            crypto::crypto_encrypt,
            crypto::crypto_decrypt,
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
            url: if cfg!(any(windows, target_os = "android")) {
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

#[test]
fn native_ipc_preserves_snowflakes_and_exchanges_encrypted_messages() {
    let (alice_app, alice) = client();
    let (_bob_app, bob) = client();
    let id = "9007199254740993"; // Cannot be represented exactly by a JavaScript Number.
    invoke(&alice, "crypto_init", json!({"user_id":id})).unwrap();
    assert_eq!(
        *alice_app
            .state::<crypto::CryptoState>()
            .user_id
            .lock()
            .unwrap(),
        9_007_199_254_740_993
    );
    assert!(invoke(&alice, "crypto_init", json!({"user_id":"invalid"})).is_err());
    assert!(invoke(
        &alice,
        "crypto_init",
        json!({"user_id":9007199254740993_i64})
    )
    .is_err());
    let bob_package = invoke(&bob, "crypto_init", json!({"user_id":"9007199254740995"})).unwrap();
    let alice_state = invoke(&alice, "crypto_create_group", json!({})).unwrap();
    let added = invoke(
        &alice,
        "crypto_add_member",
        json!({
            "group_state_hex":alice_state,"member_key_package_hex":bob_package
        }),
    )
    .unwrap();
    let bob_state = invoke(
        &bob,
        "crypto_process_welcome",
        json!({"welcome_hex":added["welcome_hex"]}),
    )
    .unwrap();
    let alice_state = added["group_state_hex"].clone();
    let a_key = invoke(
        &alice,
        "crypto_export_voice_key",
        json!({"group_state_hex":alice_state}),
    )
    .unwrap();
    let b_key = invoke(
        &bob,
        "crypto_export_voice_key",
        json!({"group_state_hex":bob_state}),
    )
    .unwrap();
    assert_eq!(a_key, b_key);

    let encrypted = invoke(
        &alice,
        "crypto_encrypt",
        json!({
            "group_state_hex":alice_state,"plaintext":"native IPC hello"
        }),
    )
    .unwrap();
    let received = invoke(
        &bob,
        "crypto_decrypt",
        json!({
            "group_state_hex":bob_state,"ciphertext_hex":encrypted["ciphertext_hex"]
        }),
    )
    .unwrap();
    assert_eq!(received["plaintext"], "native IPC hello");

    let reply = invoke(
        &bob,
        "crypto_encrypt",
        json!({
            "group_state_hex":received["group_state_hex"],"plaintext":"native IPC reply"
        }),
    )
    .unwrap();
    let received_reply = invoke(
        &alice,
        "crypto_decrypt",
        json!({
            "group_state_hex":encrypted["group_state_hex"],"ciphertext_hex":reply["ciphertext_hex"]
        }),
    )
    .unwrap();
    assert_eq!(received_reply["plaintext"], "native IPC reply");

    // File commands share the same snake_case boundary and epoch exporter.
    let file = invoke(
        &alice,
        "crypto_encrypt_file",
        json!({
            "group_state_hex":received_reply["group_state_hex"],"file_data_b64":"aGVsbG8="
        }),
    )
    .unwrap();
    let plain_file = invoke(
        &bob,
        "crypto_decrypt_file",
        json!({
            "group_state_hex":reply["group_state_hex"],"encrypted_b64":file
        }),
    )
    .unwrap();
    assert_eq!(plain_file, "aGVsbG8=");
}
