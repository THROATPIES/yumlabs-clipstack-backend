use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    #[allow(dead_code)]
    refresh_token: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    scope: Vec<String>,
    #[allow(dead_code)]
    token_type: String,
}

#[derive(Deserialize, Serialize)]
pub struct UserResponse {
    data: Vec<UserData>,
}

#[derive(Deserialize, Serialize)]
pub struct UserData {
    id: String,
    login: String,
    display_name: String,
}



#[derive(Serialize)]
pub struct CommentResponse {
    user: String,
    body: String,
}