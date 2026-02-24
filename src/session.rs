//! Session manager — tracks all logged-in sessions and switch requests.
//!
//! Switch protocol:
//!   1. Any event loop detects Ctrl+N → calls request_switch(n-1)
//!   2. That event loop returns its escape value (Back / None / false)
//!   3. Call stack unwinds naturally back to run() in main.rs
//!   4. run() calls take_switch_request() and acts on it

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

pub const MAX_SESSIONS: usize = 9;

// ── Session entry ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SessionEntry {
    pub username: String,
    pub label:    String,   // current location e.g. "Main Menu", "Documents"
}

// ── Global state ──────────────────────────────────────────────────────────────

static SESSIONS: Mutex<Vec<SessionEntry>> = Mutex::new(Vec::new());
static ACTIVE:   AtomicUsize              = AtomicUsize::new(0);
// -1 = no request, 0..8 = switch to that index, MAX_SESSIONS = new session
static SWITCH_REQUEST: AtomicI32 = AtomicI32::new(-1);

// ── Session list accessors ────────────────────────────────────────────────────

pub fn push_session(username: &str) -> usize {
    let mut s = SESSIONS.lock().unwrap();
    let idx = s.len();
    s.push(SessionEntry { username: username.to_string(), label: "Main Menu".into() });
    idx
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

pub fn active_username() -> Option<String> {
    let s = SESSIONS.lock().unwrap();
    let idx = ACTIVE.load(Ordering::Relaxed);
    s.get(idx).map(|e| e.username.clone())
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
    if v >= 0 { Some(v as usize) } else { None }
}

pub fn has_switch_request() -> bool {
    SWITCH_REQUEST.load(Ordering::Relaxed) >= 0
}
