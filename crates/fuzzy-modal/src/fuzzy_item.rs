/// TODO: docs
pub struct FuzzyItem {
    pub text: String,
}

impl FuzzyItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}
