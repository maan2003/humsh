use crossterm::event::{KeyCode, KeyEvent};

use crate::data::{Action, Keybind};

#[derive(Debug, Clone, Default)]
pub struct KeyHandler {
    current_keys: String,
}

impl KeyHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prefix(&self) -> &str {
        &self.current_keys
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        bindings: impl Iterator<Item = (Keybind, Action)>,
    ) -> crossterm::Result<Option<Action>> {
        let key = match key.code {
            KeyCode::Char('`') => return Ok(Some(Action::ToggleCmd)),
            KeyCode::Char(c) => c,
            KeyCode::Esc | KeyCode::F(9) => {
                return Ok(Some(Action::Escape));
            }
            _ => return Ok(None),
        };

        self.current_keys.push(key);
        for (key, action) in bindings {
            if key.0 == self.current_keys {
                self.current_keys = String::new();
                return Ok(Some(action));
            } else if key.0.starts_with(&self.current_keys) {
                return Ok(None);
            }
        }
        self.current_keys = String::new();
        Ok(None)
    }
}
