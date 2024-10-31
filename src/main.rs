mod env;
mod utils;
mod structs;

use std::collections::HashMap;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use utils::{exchange_code_for_token, get_user_info};


#[post("/api/start_capture")]
async fn start_capture(query: web::Query<HashMap<String, String>>) -> impl Responder {
    if let Some(display_name) = query.get("display_name") {
        HttpResponse::Ok().body(format!("Started capture for user: {}", display_name))
    } else {
        HttpResponse::BadRequest().body("Missing user ID")
    }
}

#[get("/callback")]
async fn oauth_callback(query: web::Query<HashMap<String, String>>) -> impl Responder {
    if let Some(code) = query.get("code") {
        match exchange_code_for_token(code).await {
            Ok(token) => match get_user_info(&token.access_token).await {
                Ok(user_info) => {
                    let html = format!(
                        r#"
                        <html>
                        <body>
                            <script>
                                window.opener.postMessage({{ type: 'AUTH_SUCCESS', userInfo: {} }}, 'http://localhost:3000');
                                window.close();
                            </script>
                        </body>
                        </html>
                        "#,
                        serde_json::to_string(&user_info).unwrap()
                    );
                    HttpResponse::Ok().content_type("text/html").body(html)
                },
                Err(err) => HttpResponse::InternalServerError().body(format!("Error fetching user info: {}", err)),
            },
            Err(err) => HttpResponse::InternalServerError().body(format!("Error exchanging code: {}", err)),
        }
    } else {
        HttpResponse::BadRequest().body("Missing authorization code")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(oauth_callback))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}