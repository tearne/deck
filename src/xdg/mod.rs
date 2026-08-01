//! Resolves the app's storage directories under the XDG Base Directory standard.
//!
//! Three homes, each honouring its `XDG_*_HOME` override and otherwise falling
//! back to the standard location under `$HOME`, all scoped to a `deck/` subdir:
//!
//! - config → `$XDG_CONFIG_HOME` else `~/.config`   — user configuration
//! - data   → `$XDG_DATA_HOME`   else `~/.local/share` — durable per-track data
//! - state  → `$XDG_STATE_HOME`  else `~/.local/state` — logs and session state

use std::path::PathBuf;

const APP_SUBDIR: &str = "deck";

pub(crate) fn config_dir() -> PathBuf {
    home_for("XDG_CONFIG_HOME", ".config")
}

pub(crate) fn data_dir() -> PathBuf {
    home_for("XDG_DATA_HOME", ".local/share")
}

pub(crate) fn state_dir() -> PathBuf {
    home_for("XDG_STATE_HOME", ".local/state")
}

/// The override env var wins; otherwise the standard path under `$HOME`.
/// Falls back to the current directory when `$HOME` is unset.
fn home_for(override_var: &str, home_relative: &str) -> PathBuf {
    let base = match std::env::var_os(override_var) {
        Some(x) => PathBuf::from(x),
        None => home_dir().join(home_relative),
    };
    base.join(APP_SUBDIR)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
