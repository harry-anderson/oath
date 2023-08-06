use serde::{Deserialize, Serialize};

pub static COOKIE_NAME: &str = "SESSION";

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub email: String,
}
