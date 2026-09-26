//! Context-Aware Profile Switching Service
//!
//! Evaluates the active foreground application window against configurable rules
//! and suggests or applies the optimal `ControlMode`.

use crate::infrastructure::window_tracker::ForegroundWatcher;
use crate::presentation::radial_menu::ControlMode;
use tracing::info;

/// Rule definition mapping a process name pattern to a ControlMode
#[derive(Debug, Clone)]
pub struct ContextProfileRule {
    pub process_match: &'static str,
    pub target_mode: ControlMode,
}

/// Service orchestrating automatic profile changes based on context (Single Responsibility)
pub struct ContextProfileService<W: ForegroundWatcher> {
    watcher: W,
    rules: Vec<ContextProfileRule>,
    last_switched_process: Option<String>,
}

impl<W: ForegroundWatcher> ContextProfileService<W> {
    pub fn new(watcher: W) -> Self {
        let default_rules = vec![
            // Presentation Tools -> Presenter Mode
            ContextProfileRule {
                process_match: "powerpnt.exe",
                target_mode: ControlMode::Presentation,
            },
            ContextProfileRule {
                process_match: "wps.exe",
                target_mode: ControlMode::Presentation,
            },
            ContextProfileRule {
                process_match: "acrobat.exe",
                target_mode: ControlMode::Presentation,
            },
            // Media & Browser tools -> Touchpad scrolling mode
            ContextProfileRule {
                process_match: "vlc.exe",
                target_mode: ControlMode::Touchpad,
            },
            ContextProfileRule {
                process_match: "potplayer64.exe",
                target_mode: ControlMode::Touchpad,
            },
            ContextProfileRule {
                process_match: "bilibili.exe",
                target_mode: ControlMode::Touchpad,
            },
        ];

        Self {
            watcher,
            rules: default_rules,
            last_switched_process: None,
        }
    }

    /// Evaluates current active foreground process against configured rules.
    /// Returns `Some(new_mode)` if a context transition is detected.
    pub fn evaluate_context(&mut self, enabled: bool) -> Option<ControlMode> {
        if !enabled {
            return None;
        }

        let current_process = self.watcher.get_foreground_process_name()?;

        if self.last_switched_process.as_deref() == Some(&current_process) {
            return None;
        }

        self.last_switched_process = Some(current_process.clone());

        for rule in &self.rules {
            if current_process.contains(rule.process_match) {
                info!(
                    "Context switch triggered by '{}' -> Mode: {:?}",
                    current_process, rule.target_mode
                );
                return Some(rule.target_mode);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWatcher {
        process_name: Option<String>,
    }

    impl ForegroundWatcher for MockWatcher {
        fn get_foreground_process_name(&mut self) -> Option<String> {
            self.process_name.clone()
        }
    }

    #[test]
    fn test_context_switching_rules() {
        let mut service = ContextProfileService::new(MockWatcher {
            process_name: Some("powerpnt.exe".to_string()),
        });

        // First evaluation should switch to Presentation
        let mode = service.evaluate_context(true);
        assert_eq!(mode, Some(ControlMode::Presentation));

        // Same active app should return None (deduplicated)
        let mode_again = service.evaluate_context(true);
        assert_eq!(mode_again, None);

        // Switch to media player
        service.watcher.process_name = Some("vlc.exe".to_string());
        let mode_media = service.evaluate_context(true);
        assert_eq!(mode_media, Some(ControlMode::Touchpad));

        // When disabled, should return None
        service.watcher.process_name = Some("wps.exe".to_string());
        assert_eq!(service.evaluate_context(false), None);
    }
}

