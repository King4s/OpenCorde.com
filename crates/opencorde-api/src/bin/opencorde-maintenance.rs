//! Offline operations: stop all API/bridge writers before invoking this binary.
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3600),
        opencorde_api::maintenance::run(&args),
    )
    .await;
    match result {
        Ok(Ok(report)) => println!("{report}"),
        _ => {
            // Driver errors can contain connection URLs, credentials, object keys,
            // or message content. Keep the process error surface deliberately small.
            println!(
                "{}",
                serde_json::json!({"ok": false, "error": "maintenance failed; check command, fresh destination, service access and data integrity"})
            );
            std::process::exit(1);
        }
    }
}
