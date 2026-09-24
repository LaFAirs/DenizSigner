//! Local logging: file + frontend mirror, INFO by default.
//!
//! Secrets policy: NEVER log passwords, 2FA codes, tokens, private keys,
//! cookies or full credentials. [`redact`] is applied to auth-adjacent
//! messages before they reach any sink.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tauri::{AppHandle, Emitter};
use tracing_subscriber::layer::Context;
use tracing_subscriber::{Layer, registry::LookupSpan};

/// 1=trace 2=debug 3=info 4=warn 5=error (matches frontend LogLevel).
pub static FRONTEND_LEVEL: AtomicU8 = AtomicU8::new(3);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedLogRecord {
    pub level: u8,
    pub message: String,
    pub target: Option<String>,
    pub timestamp: String,
}

/// Replaces likely-secret values with `[redacted]`, and strips URL query
/// strings (which routinely carry signed tokens) down to `?`.
/// Best-effort defense in depth: secrets must never be *passed* to logging
/// in the first place (see call sites); this only limits accidental leaks.
pub fn redact(message: &str) -> String {
    let mut out = message.to_string();
    for key in [
        "password",
        "passwd",
        "2fa",
        "two-factor",
        "tfa_code",
        "verification_code",
        "token",
        "session",
        "cookie",
        "privatekey",
        "private_key",
        "secret",
        "authorization",
        "credential",
        "set-cookie",
    ] {
        let mut search = 0;
        loop {
            let lower = out.to_lowercase();
            let Some(idx) = lower[search..].find(key) else {
                break;
            };
            let abs = search + idx;
            // Redact the remainder of the line segment after the key.
            let end = out[abs..]
                .find(['\n', ';'])
                .map(|e| abs + e)
                .unwrap_or(out.len().min(abs + 96));
            out.replace_range(abs..end, &format!("{key}=[redacted]"));
            search = abs + key.len() + 12;
            if search >= out.len() {
                break;
            }
        }
    }
    // Strip `?...` query strings (signed download URLs, auth callbacks).
    let mut stripped = String::with_capacity(out.len());
    let mut chars = out.chars();
    while let Some(c) = chars.next() {
        if c == '?' {
            // Keep the `?` marker so truncation is visible, drop the value
            // up to the next whitespace (query runs to end of the token).
            stripped.push('?');
            for c2 in chars.by_ref() {
                if c2.is_whitespace() {
                    stripped.push(c2);
                    break;
                }
            }
        } else {
            stripped.push(c);
        }
    }
    stripped
}

pub struct FrontendLoggingLayer {
    app_handle: Arc<AppHandle>,
}

impl FrontendLoggingLayer {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle: Arc::new(app_handle) }
    }
}

impl<S> Layer<S> for FrontendLoggingLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        use tracing::field::Visit;

        let metadata = event.metadata();
        let level = match *metadata.level() {
            tracing::Level::TRACE => 1u8,
            tracing::Level::DEBUG => 2u8,
            tracing::Level::INFO => 3u8,
            tracing::Level::WARN => 4u8,
            tracing::Level::ERROR => 5u8,
        };
        if level < FRONTEND_LEVEL.load(Ordering::Relaxed) {
            return;
        }

        struct Visitor {
            message: String,
            fields: Vec<String>,
        }
        impl Visitor {
            fn push(&mut self, name: &str, value: String) {
                if name == "message" {
                    self.message = value;
                } else {
                    self.fields.push(format!("{name}={value}"));
                }
            }
        }
        impl Visit for Visitor {
            fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
                self.push(f.name(), format!("{v:?}"));
            }
            fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
                self.push(f.name(), v.to_string());
            }
            fn record_bool(&mut self, f: &tracing::field::Field, v: bool) {
                self.push(f.name(), v.to_string());
            }
            fn record_i64(&mut self, f: &tracing::field::Field, v: i64) {
                self.push(f.name(), v.to_string());
            }
            fn record_u64(&mut self, f: &tracing::field::Field, v: u64) {
                self.push(f.name(), v.to_string());
            }
        }

        let mut v = Visitor { message: String::new(), fields: Vec::new() };
        event.record(&mut v);
        let raw = if v.fields.is_empty() {
            v.message
        } else {
            format!("{} ({})", v.message, v.fields.join(", "))
        };
        let record = ExtendedLogRecord {
            level,
            message: redact(&raw),
            target: Some(metadata.target().to_string()),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        };
        let _ = self.app_handle.emit("log-record", &record);
    }
}

/// Sets the frontend-visible log level (`debug` enables level 2, else 3+).
#[tauri::command]
pub fn set_log_level_debug(enabled: bool) {
    FRONTEND_LEVEL.store(if enabled { 2 } else { 3 }, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets() {
        let msg = redact("login failed password=hunter2; token=abc");
        assert!(!msg.contains("hunter2"));
        assert!(!msg.contains("abc"));
        assert!(msg.contains("[redacted]"));
    }

    #[test]
    fn redacts_2fa_tokens_and_query_secrets() {
        let msg = redact("submit 2fa=123456 for session xyz");
        assert!(!msg.contains("123456"));
        let msg = redact("GET https://cdn.example/a/b?st=ABCDEFG&x=1 done");
        assert!(!msg.contains("ABCDEFG"));
        assert!(msg.contains("https://cdn.example/a/b? done"));
        let msg = redact("authorization: Bearer ABCDEF");
        assert!(!msg.contains("ABCDEF"));
        let msg = redact("python -c pass");
        assert!(msg.contains("python -c pass"));
    }
}
