#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Syntax {
    pub typescript: bool,
    pub markup: bool,
}

impl Syntax {
    pub const fn nori() -> Self {
        Self {
            typescript: true,
            markup: true,
        }
    }
}

impl Default for Syntax {
    fn default() -> Self {
        Self::nori()
    }
}
