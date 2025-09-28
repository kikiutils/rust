#[derive(Clone)]
pub struct LoggerConfig {
    pub display_target: bool,
    pub display_level: bool,
}

// Main impl
impl LoggerConfig {
    pub fn with_target(mut self, display_target: bool) -> Self {
        self.display_target = display_target;
        self
    }

    pub fn with_level(mut self, display_level: bool) -> Self {
        self.display_level = display_level;
        self
    }
}

// Other impl
impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            display_target: false,
            display_level: true,
        }
    }
}
