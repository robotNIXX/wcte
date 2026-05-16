use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::process::Command;

const WIDGET_SERVICE: &str = "claude-usage-widget";
const ORG_CACHE_USER: &str = "org-id";

// ── sessionKey via Python3 + openssl ──────────────────────────
//
// Claude desktop app stores the cookie encrypted with AES-128-CBC.
// Key derivation: PBKDF2-HMAC-SHA1(safe_storage_key, "saltysalt", 1003, 16)
// The safe_storage_key is raw bytes from macOS Keychain ("Claude Safe Storage").
// Using Python3 + openssl handles binary key bytes correctly.

const DECRYPT_SCRIPT: &str = r#"
import sys, sqlite3, subprocess, shutil, os, tempfile
from hashlib import pbkdf2_hmac

def try_chromium(cookies_path, keychain_service):
    """Extract sessionKey from a Chromium-based browser (v10 AES-CBC)."""
    if not os.path.exists(cookies_path):
        return None
    tmp = tempfile.mktemp(suffix=".db")
    shutil.copy2(cookies_path, tmp)
    try:
        r = subprocess.run(
            ["security", "find-generic-password", "-s", keychain_service, "-w"],
            capture_output=True,
        )
        if r.returncode != 0:
            return None
        # Keychain stores the key as a base64 string; use it as-is for PBKDF2
        safe_key_b64 = r.stdout.rstrip(b"\n")
        aes_key = pbkdf2_hmac("sha1", safe_key_b64, b"saltysalt", 1003, dklen=16)

        conn = sqlite3.connect(tmp)
        row = conn.execute(
            "SELECT encrypted_value FROM cookies"
            " WHERE host_key LIKE '%claude.ai%' AND name='sessionKey' LIMIT 1"
        ).fetchone()
        conn.close()

        if not row:
            return None
        enc = bytes(row[0])
        if len(enc) < 20 or enc[:3] != b"v10":
            return None

        # v10: 3-byte prefix + 16-byte embedded IV + ciphertext
        r = subprocess.run(
            ["openssl", "enc", "-aes-128-cbc", "-d",
             "-K", aes_key.hex(), "-iv", enc[3:19].hex(),
             "-nosalt", "-nopad"],
            input=enc[19:], capture_output=True,
        )
        if r.returncode != 0 or not r.stdout:
            return None

        result = r.stdout[:-r.stdout[-1]]  # strip PKCS7 padding
        idx = result.find(b"sk-ant-")
        if idx == -1:
            return None
        return result[idx:].decode()
    except Exception:
        return None
    finally:
        try:
            os.unlink(tmp)
        except Exception:
            pass

def try_firefox():
    """Extract sessionKey from Firefox (cookies stored in plain text)."""
    profiles_dir = os.path.expanduser(
        "~/Library/Application Support/Firefox/Profiles"
    )
    if not os.path.exists(profiles_dir):
        return None
    for profile in os.listdir(profiles_dir):
        db = os.path.join(profiles_dir, profile, "cookies.sqlite")
        if not os.path.exists(db):
            continue
        tmp = tempfile.mktemp(suffix=".db")
        shutil.copy2(db, tmp)
        try:
            conn = sqlite3.connect(tmp)
            row = conn.execute(
                "SELECT value FROM moz_cookies"
                " WHERE host LIKE '%claude.ai%' AND name='sessionKey' LIMIT 1"
            ).fetchone()
            conn.close()
            if row and row[0].startswith("sk-ant-"):
                return row[0]
        except Exception:
            pass
        finally:
            try:
                os.unlink(tmp)
            except Exception:
                pass
    return None

CHROMIUM_SOURCES = [
    (
        "~/Library/Application Support/Claude/Cookies",
        "Claude Safe Storage",
    ),
    (
        "~/Library/Application Support/Google/Chrome/Default/Cookies",
        "Chrome Safe Storage",
    ),
    (
        "~/Library/Application Support/BraveSoftware/Brave-Browser/Default/Cookies",
        "Brave Browser Safe Storage",
    ),
    (
        "~/Library/Application Support/Microsoft Edge/Default/Cookies",
        "Microsoft Edge Safe Storage",
    ),
    (
        "~/Library/Application Support/Chromium/Default/Cookies",
        "Chromium Safe Storage",
    ),
]

for path, service in CHROMIUM_SOURCES:
    key = try_chromium(os.path.expanduser(path), service)
    if key:
        sys.stdout.write(key)
        sys.exit(0)

key = try_firefox()
if key:
    sys.stdout.write(key)
    sys.exit(0)

sys.exit(
    "sessionKey not found. Log in to claude.ai in one of:\n"
    "  Claude desktop app, Chrome, Brave, Edge, or Firefox — then retry."
)
"#;

fn read_session_key() -> Result<String, String> {
    let out = Command::new("python3")
        .args(["-c", DECRYPT_SCRIPT])
        .output()
        .map_err(|e| format!("python3 not found: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    String::from_utf8(out.stdout)
        .map_err(|_| "sessionKey contains non-UTF-8 bytes".to_string())
}

// ── HTTP helpers ───────────────────────────────────────────────

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

fn with_auth(rb: reqwest::RequestBuilder, session_key: &str) -> reqwest::RequestBuilder {
    rb.header("Cookie", format!("sessionKey={session_key}"))
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .header("Accept", "application/json")
        .header("Referer", "https://claude.ai/")
        .header("Origin", "https://claude.ai")
        .header("anthropic-client-platform", "web_claude_ai")
}

// ── Org ID (cached in keychain) ────────────────────────────────

#[derive(Deserialize)]
struct OrgItem {
    uuid: String,
}

async fn get_org_id(session_key: &str) -> Result<String, String> {
    if let Ok(cached) = Entry::new(WIDGET_SERVICE, ORG_CACHE_USER)
        .and_then(|e| e.get_password())
    {
        return Ok(cached);
    }

    let client = build_client()?;
    let resp = with_auth(client.get("https://claude.ai/api/organizations"), session_key)
        .send()
        .await
        .map_err(|e| format!("Network: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Auth failed ({})", resp.status()));
    }

    let orgs: Vec<OrgItem> = resp.json().await.map_err(|e| format!("Parse orgs: {e}"))?;
    let org_id = orgs
        .into_iter()
        .next()
        .map(|o| o.uuid)
        .ok_or_else(|| "No organizations found".to_string())?;

    let _ = Entry::new(WIDGET_SERVICE, ORG_CACHE_USER).and_then(|e| e.set_password(&org_id));

    Ok(org_id)
}

// ── Usage API ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct UsageWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct ApiUsage {
    five_hour: Option<UsageWindow>,
    seven_day: Option<UsageWindow>,
}

#[derive(Serialize)]
struct UsageData {
    five_hour_pct: f64,
    five_hour_resets_at: Option<String>,
    seven_day_pct: f64,
    seven_day_resets_at: Option<String>,
}

// ── Tauri command ──────────────────────────────────────────────

#[tauri::command]
async fn get_usage() -> Result<UsageData, String> {
    let session_key = read_session_key()?;
    let org_id = get_org_id(&session_key).await?;

    let client = build_client()?;
    let resp = with_auth(
        client.get(format!(
            "https://claude.ai/api/organizations/{org_id}/usage"
        )),
        &session_key,
    )
    .send()
    .await
    .map_err(|e| format!("Network: {e}"))?;

    if !resp.status().is_success() {
        if resp.status().as_u16() == 404 {
            let _ = Entry::new(WIDGET_SERVICE, ORG_CACHE_USER)
                .and_then(|e| e.delete_credential());
        }
        return Err(format!("API error {}", resp.status()));
    }

    let api: ApiUsage = resp.json().await.map_err(|e| format!("Parse: {e}"))?;

    let (fh_pct, fh_reset) = api
        .five_hour
        .map(|w| (w.utilization.unwrap_or(0.0), w.resets_at))
        .unwrap_or_default();

    let (sd_pct, sd_reset) = api
        .seven_day
        .map(|w| (w.utilization.unwrap_or(0.0), w.resets_at))
        .unwrap_or_default();

    Ok(UsageData {
        five_hour_pct: fh_pct,
        five_hour_resets_at: fh_reset,
        seven_day_pct: sd_pct,
        seven_day_resets_at: sd_reset,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_usage])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
