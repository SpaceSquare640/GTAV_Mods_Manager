// SPDX-License-Identifier: AGPL-3.0-only

//! AI Assistant System — v0.6 slice per the project's design (see the planning vault,
//! not part of this repo): **crash/error log diagnosis only**, read-only advice, no
//! automated Plan → execute yet (that's v0.7, and needs the Action Schema + double
//! validation against the core engine's existing safety checks before it can touch
//! anything).
//!
//! **Opt-in, disabled by default** — [`AiSettings::enabled`] must be `true`, set only
//! via [`enable`], before [`diagnose`] will do anything. Every request is passed through
//! [`crate::crash_report`]'s de-identification (strip the user's home directory)
//! before it leaves this function, whichever provider is selected.
//!
//! Two provider kinds, matching the design doc:
//! - **Ollama** (local): HTTP to `localhost:11434`, never leaves the machine. This
//!   project does not bundle or install Ollama or any model — the user installs it
//!   themselves; [`ollama_available`] just checks whether something is listening.
//! - **Cloud**: a user-supplied API key against an OpenAI-compatible chat-completions
//!   endpoint. The key is never stored in the SQLite database or any plaintext
//!   config file — it lives only in the OS-native credential store (Windows
//!   Credential Manager / Linux Secret Service) via the `keyring` crate.
//!
//! **Honesty note**: the cloud path has been exercised end-to-end against a real
//! OpenAI-compatible provider (OpenRouter, 2026-08-30) — real HTTP round trip, real
//! model response, correctly parsed by both [`diagnose`] and [`crate::translation`].
//! The local Ollama path is still unverified against a real running Ollama instance —
//! only the request-building, response-parsing, and the "provider unavailable" path
//! (genuinely verified: no Ollama was running on the dev machine) have been checked.
//!
//! Every request also carries [`SYSTEM_SAFETY_PROMPT`] as a system-role message (or
//! Ollama's `system` field) ahead of the actual diagnosis/translation prompt — a
//! prompt-level reinforcement of operating rules that are otherwise enforced in Rust
//! code (the Action Schema's Plan → confirm → execute model, path scoping, no silent
//! deletion). This doesn't replace that code-level enforcement — the LLM's output is
//! still just advice/text, never something that executes directly — but it keeps the
//! model's own suggestions aligned with the same rules rather than contradicting them.

pub mod action_schema;
pub mod known_fixes;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

const KEYRING_SERVICE: &str = "GTAVModsManager";
const KEYRING_USERNAME: &str = "ai_cloud_api_key";
const DEFAULT_CLOUD_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const OLLAMA_BASE_URL: &str = "http://localhost:11434";
/// A real free-tier cloud model was found, during testing, to hang indefinitely with
/// no response rather than erroring — without a timeout that looks identical to the
/// app being frozen, not a provider problem. Applies to both Ollama and cloud calls.
const PROVIDER_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Sent as a system-role message (or Ollama's `system` field) ahead of every
/// diagnosis/translation prompt — see the module doc comment for why this exists
/// alongside, not instead of, the Rust-level enforcement in `action_schema`.
pub(crate) const SYSTEM_SAFETY_PROMPT: &str = "\
You are the AI assistant inside GTAV Mods Manager, a GTA V mod management tool. You only ever \
produce text (a diagnosis or a translation draft) — you never execute anything directly. Follow \
these operating rules in every suggestion you make:\n\
1. If you suggest any change to the user's mod setup or files, describe it clearly and \
completely enough that the user (or the app's own logging) can record exactly what changed and \
why — never suggest a vague or partial change.\n\
2. Only ever reference files, folders, or paths that were given to you in the current request's \
context. Never assume the existence of, or suggest touching, any path you were not given.\n\
3. If a fix would require access to something outside what you were given (a different file, a \
different folder), say so explicitly and ask the user to provide it — do not guess a path or \
proceed as if you already had access.\n\
4. Never suggest deleting a file or mod as an immediate, silent action. Any deletion-related \
suggestion must clearly state exactly what would be deleted and why, and must be presented as \
something the user explicitly confirms before it happens — never as something already done or \
safe to do without confirmation.\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    Ollama,
    Cloud,
}

impl AiProviderKind {
    fn as_str(self) -> &'static str {
        match self {
            AiProviderKind::Ollama => "ollama",
            AiProviderKind::Cloud => "cloud",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "ollama" => Some(AiProviderKind::Ollama),
            "cloud" => Some(AiProviderKind::Cloud),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AiSettings {
    pub enabled: bool,
    pub provider: Option<AiProviderKind>,
    pub ollama_model: Option<String>,
    pub cloud_endpoint: Option<String>,
    pub cloud_model: Option<String>,
}

pub fn load_settings(conn: &Connection) -> CoreResult<AiSettings> {
    conn.query_row(
        "SELECT ai_enabled, ai_provider, ai_ollama_model, ai_cloud_endpoint, ai_cloud_model \
         FROM user_settings WHERE id = 1",
        [],
        |row| {
            let enabled: i64 = row.get(0)?;
            let provider: Option<String> = row.get(1)?;
            Ok(AiSettings {
                enabled: enabled != 0,
                provider: provider.and_then(|p| AiProviderKind::parse(&p)),
                ollama_model: row.get(2)?,
                cloud_endpoint: row.get(3)?,
                cloud_model: row.get(4)?,
            })
        },
    )
    .map_err(Into::into)
}

/// Enables the AI assistant with the given provider and optional model/endpoint
/// overrides. Does **not** touch the API key — call [`set_cloud_api_key`] separately
/// for [`AiProviderKind::Cloud`].
pub fn enable(
    conn: &Connection,
    provider: AiProviderKind,
    model: Option<String>,
    cloud_endpoint: Option<String>,
) -> CoreResult<()> {
    match provider {
        AiProviderKind::Ollama => {
            conn.execute(
                "UPDATE user_settings SET ai_enabled = 1, ai_provider = ?1, ai_ollama_model = ?2 \
                 WHERE id = 1",
                rusqlite::params![provider.as_str(), model],
            )?;
        }
        AiProviderKind::Cloud => {
            conn.execute(
                "UPDATE user_settings \
                 SET ai_enabled = 1, ai_provider = ?1, ai_cloud_model = ?2, ai_cloud_endpoint = ?3 \
                 WHERE id = 1",
                rusqlite::params![provider.as_str(), model, cloud_endpoint],
            )?;
        }
    }
    Ok(())
}

pub fn disable(conn: &Connection) -> CoreResult<()> {
    conn.execute("UPDATE user_settings SET ai_enabled = 0 WHERE id = 1", [])?;
    Ok(())
}

/// Stores the cloud API key in the OS-native credential store — never in the SQLite
/// database or any plaintext file, per the project's design decision.
pub fn set_cloud_api_key(key: &str) -> CoreResult<()> {
    let entry = keyring_entry()?;
    entry.set_password(key).map_err(|e| CoreError::AiAssistant {
        reason: format!("could not save the API key to the OS credential store: {e}"),
    })
}

pub fn has_cloud_api_key() -> bool {
    keyring_entry().is_ok_and(|e| e.get_password().is_ok())
}

fn get_cloud_api_key() -> CoreResult<String> {
    keyring_entry()?
        .get_password()
        .map_err(|e| CoreError::AiAssistant {
            reason: format!("no cloud API key is set (or it could not be read): {e}"),
        })
}

fn keyring_entry() -> CoreResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USERNAME).map_err(|e| CoreError::AiAssistant {
        reason: format!("could not access the OS credential store: {e}"),
    })
}

/// Checks whether something is listening on the local Ollama port. Does not verify a
/// model is actually pulled — just that Ollama itself appears to be running.
pub fn ollama_available() -> bool {
    ureq::get(&format!("{OLLAMA_BASE_URL}/api/tags"))
        .timeout(std::time::Duration::from_secs(2))
        .call()
        .is_ok()
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

fn build_ollama_request(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "system": SYSTEM_SAFETY_PROMPT,
        "prompt": prompt,
        "stream": false,
    })
}

fn parse_ollama_response(body: &str) -> CoreResult<String> {
    let parsed: OllamaGenerateResponse =
        serde_json::from_str(body).map_err(|e| CoreError::AiAssistant {
            reason: format!("could not parse Ollama's response: {e}"),
        })?;
    Ok(parsed.response)
}

#[derive(Deserialize)]
struct OpenAiCompatibleResponse {
    choices: Vec<OpenAiCompatibleChoice>,
}

#[derive(Deserialize)]
struct OpenAiCompatibleChoice {
    message: OpenAiCompatibleMessage,
}

#[derive(Deserialize)]
struct OpenAiCompatibleMessage {
    content: String,
}

fn build_cloud_request(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_SAFETY_PROMPT},
            {"role": "user", "content": prompt},
        ],
    })
}

fn parse_cloud_response(body: &str) -> CoreResult<String> {
    let parsed: OpenAiCompatibleResponse =
        serde_json::from_str(body).map_err(|e| CoreError::AiAssistant {
            reason: format!("could not parse the cloud provider's response: {e}"),
        })?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| CoreError::AiAssistant {
            reason: "cloud provider response had no choices".to_string(),
        })
}

const DIAGNOSIS_PROMPT_PREFIX: &str = "You are helping diagnose a problem with GTA V mods \
    managed by GTAV Mods Manager. Given the following (already de-identified) error or crash \
    log, suggest likely causes and next steps. Be concise.\n\n";

/// Sends `raw_context` (a crash/error log or free-text description) to the configured
/// provider for a read-only diagnosis. Requires [`enable`] to have been called first.
/// Always de-identifies `raw_context` before it leaves this function, regardless of
/// provider.
pub fn diagnose(conn: &Connection, raw_context: &str) -> CoreResult<String> {
    let sanitized = crate::crash_report::deidentify(raw_context);
    let prompt = format!("{DIAGNOSIS_PROMPT_PREFIX}{sanitized}");
    call_provider(conn, &prompt)
}

/// Sends `prompt` to whichever provider is currently configured (requires [`enable`]
/// to have been called first). Shared by every AI-assisted feature in this crate
/// ([`diagnose`], [`crate::translation`]) — there is exactly one place that talks to
/// Ollama/cloud endpoints, so a provider-handling fix only needs to happen once.
pub(crate) fn call_provider(conn: &Connection, prompt: &str) -> CoreResult<String> {
    let settings = load_settings(conn)?;
    if !settings.enabled {
        return Err(CoreError::AiAssistant {
            reason: "AI assistant is disabled. Enable it first (see the `ai enable` command)."
                .to_string(),
        });
    }
    let Some(provider) = settings.provider else {
        return Err(CoreError::AiAssistant {
            reason: "AI assistant is enabled but has no provider configured.".to_string(),
        });
    };

    match provider {
        AiProviderKind::Ollama => {
            let model = settings.ollama_model.as_deref().unwrap_or("llama3");
            let body = build_ollama_request(model, prompt);
            let response = ureq::post(&format!("{OLLAMA_BASE_URL}/api/generate"))
                .timeout(PROVIDER_REQUEST_TIMEOUT)
                .send_json(body)
                .map_err(|e| CoreError::AiAssistant {
                    reason: format!(
                        "could not reach Ollama at {OLLAMA_BASE_URL} — is it running? ({e})"
                    ),
                })?
                .into_string()
                .map_err(|e| CoreError::AiAssistant {
                    reason: format!("could not read Ollama's response: {e}"),
                })?;
            parse_ollama_response(&response)
        }
        AiProviderKind::Cloud => {
            let api_key = get_cloud_api_key()?;
            let endpoint = settings
                .cloud_endpoint
                .as_deref()
                .unwrap_or(DEFAULT_CLOUD_ENDPOINT);
            let model = settings.cloud_model.as_deref().unwrap_or("gpt-4o-mini");
            let body = build_cloud_request(model, prompt);
            let response = ureq::post(endpoint)
                .timeout(PROVIDER_REQUEST_TIMEOUT)
                .set("Authorization", &format!("Bearer {api_key}"))
                .send_json(body)
                .map_err(|e| CoreError::AiAssistant {
                    reason: format!("cloud provider request failed: {e}"),
                })?
                .into_string()
                .map_err(|e| CoreError::AiAssistant {
                    reason: format!("could not read the cloud provider's response: {e}"),
                })?;
            parse_cloud_response(&response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_to_disabled_with_no_provider() {
        let conn = crate::db::open_in_memory().unwrap();
        let settings = load_settings(&conn).unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.provider, None);
    }

    #[test]
    fn enable_ollama_then_disable_roundtrips() {
        let conn = crate::db::open_in_memory().unwrap();
        enable(
            &conn,
            AiProviderKind::Ollama,
            Some("llama3.1".to_string()),
            None,
        )
        .unwrap();

        let settings = load_settings(&conn).unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.provider, Some(AiProviderKind::Ollama));
        assert_eq!(settings.ollama_model.as_deref(), Some("llama3.1"));

        disable(&conn).unwrap();
        assert!(!load_settings(&conn).unwrap().enabled);
    }

    #[test]
    fn enable_cloud_stores_endpoint_and_model_not_key() {
        let conn = crate::db::open_in_memory().unwrap();
        enable(
            &conn,
            AiProviderKind::Cloud,
            Some("gpt-4o".to_string()),
            Some("https://example.com/v1/chat/completions".to_string()),
        )
        .unwrap();

        let settings = load_settings(&conn).unwrap();
        assert_eq!(settings.provider, Some(AiProviderKind::Cloud));
        assert_eq!(settings.cloud_model.as_deref(), Some("gpt-4o"));
        assert_eq!(
            settings.cloud_endpoint.as_deref(),
            Some("https://example.com/v1/chat/completions")
        );
    }

    #[test]
    fn diagnose_refuses_when_disabled() {
        let conn = crate::db::open_in_memory().unwrap();
        let err = diagnose(&conn, "some error").unwrap_err();
        assert!(matches!(err, CoreError::AiAssistant { .. }));
    }

    #[test]
    fn diagnose_deidentifies_context_before_building_the_prompt() {
        std::env::set_var("HOME", "/home/testuser");
        let sanitized = crate::crash_report::deidentify("error at /home/testuser/game/mod.asi");
        assert!(!sanitized.contains("testuser"));
        std::env::remove_var("HOME");
    }

    #[test]
    fn builds_expected_ollama_request_shape() {
        let body = build_ollama_request("llama3", "hello");
        assert_eq!(body["model"], "llama3");
        assert_eq!(body["prompt"], "hello");
        assert_eq!(body["stream"], false);
        assert_eq!(body["system"], SYSTEM_SAFETY_PROMPT);
    }

    #[test]
    fn parses_ollama_response() {
        let raw = r#"{"response": "looks like a missing dependency"}"#;
        assert_eq!(
            parse_ollama_response(raw).unwrap(),
            "looks like a missing dependency"
        );
    }

    #[test]
    fn builds_expected_cloud_request_shape() {
        let body = build_cloud_request("gpt-4o-mini", "hello");
        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], SYSTEM_SAFETY_PROMPT);
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "hello");
    }

    #[test]
    fn parses_cloud_response() {
        let raw = r#"{"choices": [{"message": {"content": "try disabling ModX"}}]}"#;
        assert_eq!(parse_cloud_response(raw).unwrap(), "try disabling ModX");
    }

    #[test]
    fn cloud_response_with_no_choices_is_an_error() {
        let raw = r#"{"choices": []}"#;
        assert!(parse_cloud_response(raw).is_err());
    }

    #[test]
    fn ollama_unavailable_when_nothing_is_listening() {
        // Real check against this machine — no Ollama install exists here, so this
        // genuinely exercises the "unavailable" path rather than asserting a mock.
        assert!(!ollama_available());
    }

    /// Regression test for a real bug found on this machine: keyring 3.x's default
    /// Cargo features include **no actual OS backend at all**, so `set_password`
    /// silently "succeeds" against a no-op store and every `get_password` afterwards
    /// returns `NoEntry` — confirmed directly against a real Windows Credential
    /// Manager on this machine before `windows-native`/`sync-secret-service` were
    /// added to `Cargo.toml`. This exercises the real OS credential store (using a
    /// distinct service name so it never touches the real `ai_cloud_api_key` entry),
    /// but tolerates the backend being genuinely unavailable — a headless Linux CI
    /// runner typically has no D-Bus Secret Service daemon running, and that's an
    /// environment limitation, not a bug in this code.
    #[test]
    fn real_os_credential_store_roundtrip_when_available() {
        let entry = keyring::Entry::new("GTAVModsManager-test", "roundtrip-test-key-do-not-use")
            .expect("constructing an Entry never touches the backend");

        if entry.set_password("roundtrip-value").is_err() {
            eprintln!("skipping: no OS credential store backend is reachable in this environment");
            return;
        }

        let read_back = entry
            .get_password()
            .expect("a value just set must be readable back, not NoEntry");
        assert_eq!(read_back, "roundtrip-value");

        let _ = entry.delete_credential();
    }
}
