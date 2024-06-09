use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub digital: String,
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
    pub percent: f32,
    pub name: String,
    pub text: String,
    pub total_seconds: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub total_seconds: usize,
    pub languages: Vec<LanguageInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPayload {
    pub data: UserInfo,
}
