use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CustomError(pub String);

impl CustomError {
    pub fn new(e: &str) -> Box<Self> {
        Box::new(CustomError::from(e))
    }

}

impl From<&str> for CustomError {
    fn from(value: &str) -> Self {
        CustomError(value.to_owned())
    }
}

impl std::error::Error for CustomError {
    // this implementation required `Debug` and `Display` traits
}

impl std::fmt::Display for CustomError {
    /// Display the error struct as a JSON string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

