use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};

use crate::data::{Callback, Keybind};

use super::Context;

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

    fn reset(&mut self) {
        self.current_keys.clear();
    }

    pub fn handle_key<'a>(
        &mut self,
        key: KeyEvent,
        bindings: impl Iterator<Item = (&'a Keybind, &'a Arc<dyn Callback>)>,
    ) -> crossterm::Result<Option<Arc<dyn Callback>>> {
        let key = match key.code {
            KeyCode::Char('`') => {
                self.reset();
                return Ok(Some(Arc::new(|mut ctx: Context| ctx.toggle_cmd())));
            }
            KeyCode::Char(c) => c,
            KeyCode::Esc | KeyCode::F(9) => {
                self.reset();
                return Ok(Some(Arc::new(|mut ctx: Context| {
                    if ctx.showing_cmd() {
                        ctx.hide_cmd()?;
                        return Ok(());
                    }
                    if !ctx.pop_page() {
                        ctx.exit();
                    }
                    Ok(())
                })));
            }
            _ => return Ok(None),
        };

        self.current_keys.push(key);
        for (key, action) in bindings {
            if key.0 == self.current_keys {
                self.current_keys = String::new();
                return Ok(Some(action.clone()));
            } else if key.0.starts_with(&self.current_keys) {
                return Ok(None);
            }
        }
        self.reset();
        Ok(None)
    }
}
