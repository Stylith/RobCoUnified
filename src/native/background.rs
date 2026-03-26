//! Background task runner for non-blocking I/O operations.
//!
//! The UI thread submits work via `BackgroundTasks::sender()` + `std::thread::spawn`,
//! then polls for results each frame with `BackgroundTasks::poll()`.

use std::sync::mpsc;

/// A completed background task result.
pub enum BackgroundResult {
    /// Nuke codes fetched from network.
    NukeCodesFetched(robcos_native_nuke_codes_app::NukeCodesView),
    /// Settings persisted to disk.
    SettingsPersisted,
    /// Repository-backed addon install/update/reinstall completed.
    RepositoryAddonInstalled {
        addon_id: String,
        status: String,
        success: bool,
    },
}

pub struct BackgroundTasks {
    tx: mpsc::Sender<BackgroundResult>,
    rx: mpsc::Receiver<BackgroundResult>,
}

impl BackgroundTasks {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    /// Returns a clone of the sender for passing into spawned threads.
    pub fn sender(&self) -> mpsc::Sender<BackgroundResult> {
        self.tx.clone()
    }

    /// Drain all completed results. Call this once per frame.
    pub fn poll(&self) -> Vec<BackgroundResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.rx.try_recv() {
            results.push(result);
        }
        results
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}
