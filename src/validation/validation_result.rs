pub struct ValidationResult {
    pub is_valid: bool,
    pub message: Option<String>,
}

impl ValidationResult {
    pub fn new(is_valid: bool) -> Self {
        Self {
            is_valid,
            message: None,
        }
    }

    pub fn with_message(is_valid: bool, message: impl Into<String>) -> Self {
        Self {
            is_valid,
            message: Some(message.into()),
        }
    }
}