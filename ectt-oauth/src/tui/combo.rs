use std::fmt::Display;

use crossterm::event::{KeyCode, KeyModifiers};

pub struct KeyCombo {
    codes: Vec<KeyCode>,
    modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new() -> Self {
        Self {
            codes: vec![],
            modifiers: KeyModifiers::empty(),
        }
    }

    pub fn with_code(mut self, code: KeyCode) -> Self {
        self.codes.push(code);
        self
    }

    pub fn with_modifier(mut self, modifier: KeyModifiers) -> Self {
        self.modifiers = self.modifiers.union(modifier);
        self
    }
}

impl Display for KeyCombo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        f.write_str(&self.modifiers.to_string())?;
        let mut first = true;
        for code in &self.codes {
            if first {
                first = false;
                f.write_str(&code.to_string())?;
                continue;
            }
            f.write_str("+")?;
            f.write_str(&code.to_string())?;
        }
        f.write_str("]")?;
        Ok(())
    }
}
