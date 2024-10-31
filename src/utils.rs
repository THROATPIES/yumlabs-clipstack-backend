use reqwest::Client;

use crate::{env::{REDIRECT_URI, TWITCH_CLIENT_ID, TWITCH_CLIENT_SECRET}, structs::{AuthResponse, UserResponse}};

pub async fn exchange_code_for_token(code: &str) -> Result<AuthResponse, reqwest::Error> {
    let client = Client::new();
    let params = [
        ("client_id", TWITCH_CLIENT_ID),
        ("client_secret", TWITCH_CLIENT_SECRET),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", REDIRECT_URI),
    ];

    let res = client
        .post("https://id.twitch.tv/oauth2/token")
        .form(&params)
        .send()
        .await?
        .json::<AuthResponse>()
        .await?;

    Ok(res)
}

pub async fn get_user_info(access_token: &str) -> Result<UserResponse, reqwest::Error> {
    let client = Client::new();
    
    let res = client
        .get("https://api.twitch.tv/helix/users")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", TWITCH_CLIENT_ID)
        .send()
        .await?
        .json::<UserResponse>()
        .await?;

    Ok(res)
}


