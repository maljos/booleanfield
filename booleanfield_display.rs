use std::fmt;

/// Display configuration for boolean fields
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooleanDisplayConfig<T: fmt::Display + Clone + 'static> {
    /// Display value for false
    pub false_display: T,
    /// Display value for true
    pub true_display: T,
    /// Display value for NULL (if allowed)
    pub null_display: Option<T>,
}

/// Handles display operations for boolean fields
pub struct BooleanDisplay<'a, T: fmt::Display + Clone + 'static> {
    value: Option<bool>,
    config: &'a BooleanDisplayConfig<T>,
}

impl<'a, T: fmt::Display + Clone + 'static> BooleanDisplay<'a, T> {
    pub fn new(value: Option<bool>, config: &'a BooleanDisplayConfig<T>) -> Self {
        Self { value, config }
    }

    /// Returns the display value for the current field value
    pub fn display_value(&self) -> String {
        match self.value {
            Some(true) => self.config.true_display.to_string(),
            Some(false) => self.config.false_display.to_string(),
            None => self
                .config
                .null_display
                .as_ref()
                .map_or_else(|| "NULL".to_string(), |v| v.to_string()),
        }
    }
}