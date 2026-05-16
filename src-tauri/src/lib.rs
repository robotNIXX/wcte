use keyring::Entry;
use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "claude-token-counter";
const KEYRING_USER: &str = "anthropic-api-key";
const COUNT_URL: &str = "https://api.anthropic.com/v1/messages/count_tokens";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct CountRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Deserialize)]
struct CountResponse {
    input_tokens: u32,
}

fn entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| format!("keyring init: {e}"))
}

#[tauri::command]
fn save_api_key(key: String) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }
    entry()?
        .set_password(key)
        .map_err(|e| format!("save key: {e}"))
}

#[tauri::command]
fn has_api_key() -> bool {
    entry().and_then(|e| e.get_password().map_err(|x| x.to_string())).is_ok()
}

#[tauri::command]
fn clear_api_key() -> Result<(), String> {
    let e = entry()?;
    match e.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(format!("clear key: {err}")),
    }
}

#[tauri::command]
async fn count_tokens(text: String, model: String) -> Result<u32, String> {
    if text.trim().is_empty() {
        return Ok(0);
    }
    let api_key = entry()?
        .get_password()
        .map_err(|_| "API key not set — open Settings to add one.".to_string())?;

    let body = CountRequest {
        model: &model,
        messages: vec![Message {
            role: "user",
            content: &text,
        }],
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .post(COUNT_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API {status}: {body}"));
    }

    let parsed: CountResponse = resp
        .json()
        .await
        .map_err(|e| format!("parse response: {e}"))?;
    Ok(parsed.input_tokens)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            save_api_key,
            has_api_key,
            clear_api_key,
            count_tokens
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
