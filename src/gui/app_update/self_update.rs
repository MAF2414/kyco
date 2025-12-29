use super::super::app::KycoApp;
use super::super::update::{UpdateInfo, UpdateStatus};

impl KycoApp {
    /// Poll the update checker and return update info if available.
    pub(crate) fn poll_update_checker(&mut self) -> Option<UpdateInfo> {
        match self.update_checker.poll() {
            UpdateStatus::UpdateAvailable(info) => Some(info.clone()),
            _ => None,
        }
    }

    /// Handle update install request if pending, and poll for install results.
    /// Returns true if an install was started or is in progress.
    pub(crate) fn handle_update_install(&mut self, update_info: Option<&UpdateInfo>) {
        // Handle install request
        if matches!(
            self.update_install_status,
            super::super::status_bar::InstallStatus::InstallRequested
        ) {
            if let Some(info) = update_info {
                self.update_install_status = super::super::status_bar::InstallStatus::Installing;
                let (tx, rx) = std::sync::mpsc::channel();
                self.update_install_rx = Some(rx);
                let info_clone = info.clone();
                std::thread::spawn(move || {
                    let result = super::super::update::install_update(&info_clone);
                    let _ = tx.send(result);
                });
            }
        }

        // Poll install result if we're installing
        if let Some(rx) = &self.update_install_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(msg) => {
                        self.update_install_status =
                            super::super::status_bar::InstallStatus::Success(msg)
                    }
                    Err(err) => {
                        self.update_install_status =
                            super::super::status_bar::InstallStatus::Error(err)
                    }
                }
                self.update_install_rx = None;
            }
        }
    }
}
