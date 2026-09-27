//! User-selectable actions. OS key codes stay at the input boundary.
use super::models::ControlMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonAction {
    LeftClick,
    RightClick,
    StartMenu,
    ShowDesktop,
    PreviousPage,
    NextPage,
    PlayPause,
    Disabled,
}

impl ButtonAction {
    pub const ALL: [Self; 8] = [
        Self::LeftClick,
        Self::RightClick,
        Self::StartMenu,
        Self::ShowDesktop,
        Self::PreviousPage,
        Self::NextPage,
        Self::PlayPause,
        Self::Disabled,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeBindings {
    pub trigger: ButtonAction,
    pub touchpad: ButtonAction,
    pub back: ButtonAction,
    pub home: ButtonAction,
}

impl ModeBindings {
    pub fn defaults(mode: ControlMode) -> Self {
        use ButtonAction::*;
        match mode {
            ControlMode::Presentation => Self {
                trigger: NextPage,
                touchpad: PlayPause,
                back: PreviousPage,
                home: Disabled,
            },
            ControlMode::Touchpad => Self {
                trigger: LeftClick,
                touchpad: LeftClick,
                back: RightClick,
                home: ShowDesktop,
            },
            _ => Self {
                trigger: LeftClick,
                touchpad: LeftClick,
                back: RightClick,
                home: StartMenu,
            },
        }
    }

    pub fn has_duplicate_shortcuts(&self) -> bool {
        let actions = [self.trigger, self.touchpad, self.back, self.home];
        actions.iter().enumerate().any(|(index, action)| {
            !matches!(action, ButtonAction::Disabled | ButtonAction::LeftClick)
                && actions[..index].contains(action)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ButtonBindings {
    pub mouse: ModeBindings,
    pub touchpad: ModeBindings,
    pub presenter: ModeBindings,
}

impl Default for ButtonBindings {
    fn default() -> Self {
        Self {
            mouse: ModeBindings::defaults(ControlMode::Mouse),
            touchpad: ModeBindings::defaults(ControlMode::Touchpad),
            presenter: ModeBindings::defaults(ControlMode::Presentation),
        }
    }
}

impl ButtonBindings {
    pub fn for_mode(&self, mode: ControlMode) -> &ModeBindings {
        match mode {
            ControlMode::Touchpad => &self.touchpad,
            ControlMode::Presentation => &self.presenter,
            _ => &self.mouse,
        }
    }

    pub fn for_mode_mut(&mut self, mode: ControlMode) -> &mut ModeBindings {
        match mode {
            ControlMode::Touchpad => &mut self.touchpad,
            ControlMode::Presentation => &mut self.presenter,
            _ => &mut self.mouse,
        }
    }
}
