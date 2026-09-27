//! A single output gate protects the desktop during pause, calibration and preview.
use crate::{
    domain::{
        input::{InputAction, InputMapper},
        models::{InputPreview, OutputTarget},
    },
    infrastructure::input_simulator::InputSimulator,
};

#[derive(Default)]
pub struct InputOutput {
    pub target: OutputTarget,
    pub preview: InputPreview,
    desktop_left_held: bool,
}

impl InputOutput {
    pub fn set_target(
        &mut self,
        target: OutputTarget,
        mapper: &mut InputMapper,
    ) -> anyhow::Result<()> {
        self.target = target;
        self.preview = InputPreview::default();
        // Only release buttons that this output gate actually pressed on the desktop.
        // Preview and paused processing can also hold mapper buttons; those holds are local.
        mapper.reset();
        if self.desktop_left_held {
            InputSimulator::new().execute(InputAction::Left(false))?;
            self.desktop_left_held = false;
        }
        Ok(())
    }

    pub fn dispatch(&mut self, actions: Vec<InputAction>) -> anyhow::Result<()> {
        match self.target {
            OutputTarget::Desktop => {
                let simulator = InputSimulator::new();
                for action in actions {
                    simulator.execute(action.clone())?;
                    if let InputAction::Left(held) = action {
                        self.desktop_left_held = held;
                    }
                }
            }
            OutputTarget::Preview => self.preview.record(&actions),
            OutputTarget::Paused => {}
        }
        Ok(())
    }
}

impl InputPreview {
    pub fn record(&mut self, actions: &[InputAction]) {
        for action in actions {
            match action {
                InputAction::Move(x, y) => {
                    self.x = self.x.saturating_add(*x).clamp(-300, 300);
                    self.y = self.y.saturating_add(*y).clamp(-300, 300);
                }
                InputAction::Left(true) | InputAction::RightClick => {
                    self.clicks = self.clicks.saturating_add(1)
                }
                InputAction::Scroll(value) => self.scroll = self.scroll.saturating_add(*value),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_preview_discards_local_drag_without_desktop_release() -> anyhow::Result<()> {
        use crate::domain::{models::ControllerData, settings::Settings};
        let mut mapper = InputMapper::default();
        let actions = mapper.process(
            &mut ControllerData {
                trigger_button: true,
                ..Default::default()
            },
            &Settings::default(),
            std::time::Instant::now(),
        );
        assert_eq!(actions, vec![InputAction::Left(true)]);
        let mut output = InputOutput {
            target: OutputTarget::Preview,
            ..Default::default()
        };
        output.dispatch(actions)?;
        // No desktop hold exists, so this transition needs no Windows input injection.
        output.set_target(OutputTarget::Paused, &mut mapper)?;
        assert!(mapper.reset().is_empty());
        assert!(!output.desktop_left_held);
        Ok(())
    }

    #[test]
    fn preview_and_pause_never_require_windows_injection() -> anyhow::Result<()> {
        let actions = vec![
            InputAction::Move(7, -9),
            InputAction::Left(true),
            InputAction::Key(0x5B),
        ];
        let mut output = InputOutput {
            target: OutputTarget::Preview,
            ..Default::default()
        };
        output.dispatch(actions.clone())?;
        assert_eq!(
            (output.preview.x, output.preview.y, output.preview.clicks),
            (7, -9, 1)
        );
        output.target = OutputTarget::Paused;
        output.dispatch(actions)?;
        assert_eq!(output.preview.x, 7);
        Ok(())
    }
}
