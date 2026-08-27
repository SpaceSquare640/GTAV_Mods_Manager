// SPDX-License-Identifier: AGPL-3.0-only

//! Builds a pre-filled GitHub Issue draft for crash/error reporting — per
//! `PRIVACY.md`, this is opt-in and never submits anything automatically: it only
//! produces text and a URL; the user must review and click "Submit new issue"
//! themselves on GitHub. Nothing here makes a network call.
//!
//! De-identification: the draft strips the current user's home directory prefix from
//! any path it appears in (the most common way a raw error message would otherwise
//! leak a username), and includes only app version, OS, and the error text — no
//! machine identifiers.

const REPO_URL: &str = "https://github.com/SpaceSquare640/GTAV_Mods_Manager";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashReportDraft {
    pub title: String,
    pub body: String,
    /// A GitHub "new issue" URL with `title`/`body`/`labels` pre-filled via query
    /// parameters. Opening it shows a draft the user can edit before submitting;
    /// nothing is sent until they click GitHub's own submit button.
    pub github_issue_url: String,
}

/// Replaces the current user's home directory (if it appears) with `<home>` in `text`.
fn deidentify(text: &str) -> String {
    let mut text = text.to_string();
    for home in [
        std::env::var("USERPROFILE").ok(),
        std::env::var("HOME").ok(),
    ]
    .into_iter()
    .flatten()
    {
        if !home.is_empty() {
            text = text.replace(&home, "<home>");
            // Also cover the other slash style, in case paths got normalized.
            text = text.replace(&home.replace('\\', "/"), "<home>");
        }
    }
    text
}

/// Minimal percent-encoding for URL query parameters — just enough for the printable
/// ASCII + newlines that error text and titles actually contain, without pulling in
/// a URL-encoding crate for this one call site.
fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Builds a crash report draft from `app_version` (e.g. `env!("CARGO_PKG_VERSION")`),
/// `os` (e.g. `std::env::consts::OS`), and the error's display text.
pub fn build(app_version: &str, os: &str, error_message: &str) -> CrashReportDraft {
    let sanitized_error = deidentify(error_message);
    let first_line = sanitized_error.lines().next().unwrap_or("error");
    let title = format!("Crash report: {first_line}");
    let body = format!(
        "**App version:** {app_version}\n**OS:** {os}\n\n**Error:**\n```\n{sanitized_error}\n```\n\n\
         <!-- Please add any extra context about what you were doing before this happened. -->\n"
    );

    let github_issue_url = format!(
        "{REPO_URL}/issues/new?title={}&body={}&labels=crash-report",
        percent_encode(&title),
        percent_encode(&body),
    );

    CrashReportDraft {
        title,
        body,
        github_issue_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_home_directory_from_error_text() {
        std::env::set_var("HOME", "/home/testuser");
        let draft = build(
            "0.1.0",
            "linux",
            "failed to read /home/testuser/game/mod.asi",
        );
        assert!(!draft.body.contains("testuser"));
        assert!(draft.body.contains("<home>/game/mod.asi"));
        std::env::remove_var("HOME");
    }

    #[test]
    fn includes_version_and_os() {
        let draft = build("1.2.3", "windows", "boom");
        assert!(draft.body.contains("1.2.3"));
        assert!(draft.body.contains("windows"));
    }

    #[test]
    fn url_is_percent_encoded_and_well_formed() {
        let draft = build("0.1.0", "windows", "error: file not found\nsecond line");
        assert!(draft
            .github_issue_url
            .starts_with("https://github.com/SpaceSquare640/GTAV_Mods_Manager/issues/new?title="));
        assert!(!draft.github_issue_url.contains(' '));
        assert!(!draft.github_issue_url.contains('\n'));
        assert!(draft.github_issue_url.contains("labels=crash-report"));
    }

    #[test]
    fn title_is_derived_from_first_line_only() {
        let draft = build("0.1.0", "windows", "first line\nsecond line\nthird line");
        assert_eq!(draft.title, "Crash report: first line");
    }
}
