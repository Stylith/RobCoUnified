//! Session manager — tracks all logged-in sessions and switch requests.
//!
//! Switch protocol:
//!   1. Any event loop detects Ctrl+N → calls request_switch(n-1)
//!   2. That event loop returns its escape value (Back / None / false)
//!   3. Call stack unwinds naturally back to run() in main.rs
//!   4. run() calls take_switch_request() and acts on it

use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub const MAX_SESSIONS: usize = 9;

// ── Session entry ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SessionEntry {
    pub username: String,
    pub label: String, // current location e.g. "Main Menu", "Documents"
    pub default_mode_pending: bool,
}

// ── Global state ──────────────────────────────────────────────────────────────

static SESSIONS: Mutex<Vec<SessionEntry>> = Mutex::new(Vec::new());
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
// -1 = no request, 0..8 = switch to that index, MAX_SESSIONS = new session
static SWITCH_REQUEST: AtomicI32 = AtomicI32::new(-1);

// ── Session list accessors ────────────────────────────────────────────────────

pub fn push_session_with_default_mode(username: &str, default_mode_pending: bool) -> usize {
    let mut s = SESSIONS.lock().unwrap();
    let idx = s.len();
    s.push(SessionEntry {
        username: username.to_string(),
        label: "Main Menu".into(),
        default_mode_pending,
    });
    idx
}

pub fn push_session(username: &str) -> usize {
    push_session_with_default_mode(username, true)
}

pub fn clear_sessions() {
    SESSIONS.lock().unwrap().clear();
    ACTIVE.store(0, Ordering::Relaxed);
}

pub fn session_count() -> usize {
    SESSIONS.lock().unwrap().len()
}

pub fn get_sessions() -> Vec<SessionEntry> {
    SESSIONS.lock().unwrap().clone()
}

pub fn active_idx() -> usize {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn set_active(idx: usize) {
    ACTIVE.store(idx, Ordering::Relaxed);
}

/// Close the active session and return the removed index.
/// The new active session becomes the previous one when possible.
#[allow(dead_code)]
pub fn close_active_session() -> Option<usize> {
    let mut s = SESSIONS.lock().unwrap();
    if s.is_empty() {
        return None;
    }
    let active = ACTIVE.load(Ordering::Relaxed).min(s.len().saturating_sub(1));
    s.remove(active);

    if s.is_empty() {
        ACTIVE.store(0, Ordering::Relaxed);
    } else {
        let new_active = active.saturating_sub(1).min(s.len().saturating_sub(1));
        ACTIVE.store(new_active, Ordering::Relaxed);
    }
    Some(active)
}

pub fn active_username() -> Option<String> {
    let s = SESSIONS.lock().unwrap();
    let idx = ACTIVE.load(Ordering::Relaxed);
    s.get(idx).map(|e| e.username.clone())
}

/// Consume the "default open mode pending" flag for the active session.
/// Returns true exactly once per session lifetime.
pub fn take_default_mode_pending_for_active() -> bool {
    let mut s = SESSIONS.lock().unwrap();
    let idx = ACTIVE.load(Ordering::Relaxed);
    if let Some(e) = s.get_mut(idx) {
        let was_pending = e.default_mode_pending;
        e.default_mode_pending = false;
        return was_pending;
    }
    false
}

/// Update the label for the currently active session (call from menus).
#[allow(dead_code)]
pub fn set_label(label: &str) {
    let mut s = SESSIONS.lock().unwrap();
    let idx = ACTIVE.load(Ordering::Relaxed);
    if let Some(e) = s.get_mut(idx) {
        e.label = label.to_string();
    }
}

// ── Switch request ────────────────────────────────────────────────────────────

/// Request a switch to session index `target` (0-based).
/// MAX_SESSIONS means "open a new session".
pub fn request_switch(target: usize) {
    SWITCH_REQUEST.store(target as i32, Ordering::SeqCst);
}

/// Consume and return the pending switch request, if any.
pub fn take_switch_request() -> Option<usize> {
    let v = SWITCH_REQUEST.swap(-1, Ordering::SeqCst);
    if v >= 0 {
        Some(v as usize)
    } else {
        None
    }
}

pub fn has_switch_request() -> bool {
    SWITCH_REQUEST.load(Ordering::Relaxed) >= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_request_roundtrip() {
        clear_sessions();
        request_switch(3);
        assert!(has_switch_request());
        assert_eq!(take_switch_request(), Some(3));
        assert!(!has_switch_request());
        assert_eq!(take_switch_request(), None);
    }

    #[test]
    fn default_mode_pending_consumed_once() {
        clear_sessions();
        let idx = push_session_with_default_mode("admin", true);
        set_active(idx);
        assert!(take_default_mode_pending_for_active());
        assert!(!take_default_mode_pending_for_active());
    }

    #[test]
    fn close_active_session_picks_previous() {
        clear_sessions();
        push_session("u1");
        push_session("u2");
        push_session("u3");
        push_session("u4");
        set_active(2);
        let removed = close_active_session();
        assert_eq!(removed, Some(2));
        assert_eq!(session_count(), 3);
        assert_eq!(active_idx(), 1);
        assert_eq!(active_username().as_deref(), Some("u2"));
    }
}
