mod env;
mod utils;
mod structs;

use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}};
use actix::{Actor, Addr, Handler, Message, StreamHandler, AsyncContext};
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use actix::spawn;
use actix_web_actors::ws;
use twitch_comment_stream::TwitchCommentStream;
use utils::{exchange_code_for_token, get_user_info};

#[post("/api/start_capture/{display_name}")]
async fn start_capture(
    path: web::Path<String>,
    data: web::Data<AppState>,
) -> impl Responder {
    let display_name = path.into_inner();
    let app_state = data.clone();

    // Spawn a new task to monitor the Twitch channel
    spawn(async move {
        app_state.start_monitoring(display_name).await;
    });

    HttpResponse::Ok().body("Started monitoring Twitch channel")
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

struct AppState {
    url_storage: Arc<Mutex<VecDeque<String>>>,
    ws_clients: Arc<Mutex<Vec<Addr<WsSession>>>>,
}

impl AppState {
    fn add_url(&self, url: String) {
         // Gather the Slug from the URL, examine it and remove the ? if it exists after the slug
        let slug = url
            .split('/')
            .last()
            .unwrap()
            .split('?')
            .next()
            .unwrap()
            .to_string();
        // what will slug return if the url is https://www.twitch.tv/lord_kebun/clip/MoralAbnegateYamSMOrc-K8NcO4NIEL_JCyKA?filter=clips&range=7d&sort=time 
        // turn it into twitches embedded URL 
        let url_reformed = format!("https://clips.twitch.tv/embed?clip={}", slug);

        let mut storage = self.url_storage.lock().unwrap();
        if storage.len() >= 100 {
            storage.pop_front();
        }
        storage.push_back(url_reformed.clone());

        // Notify all WebSocket clients of the new clip URL
        self.notify_clients(url_reformed);
    }

    fn notify_clients(&self, message: String) {
        let clients = self.ws_clients.lock().unwrap();
        for client in clients.iter() {
            client.do_send(BroadcastMessage(message.clone())); // Remove `.ok()` here
        }
    }

    async fn start_monitoring(&self, channel_name: String) {
        let mut stream = TwitchCommentStream::new(channel_name.clone());
        if let Err(e) = stream.connect().await {
            println!("Failed to connect to Twitch: {:?}", e);
            return;
        }
        let cloned_name = channel_name.clone();
        println!("Started monitoring Twitch channel: {}", cloned_name);
        while let Ok(comment) = stream.next().await {
            let comment = comment;
            println!("Received comment: {}", comment.body);
            if comment.body.contains("twitch") && comment.body.contains("clip") {
                println!("Does contain clip");
                let url = comment.body.clone();
                self.add_url(url.clone());
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct BroadcastMessage(String);

struct WsSession {
    state: Arc<Mutex<VecDeque<String>>>,
    ws_clients: Arc<Mutex<Vec<Addr<WsSession>>>>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.ws_clients.lock().unwrap().push(addr);
    }
    

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address(); // Get the actor address
        self.ws_clients.lock().unwrap().retain(|client| client != &addr);
    }
}

impl Handler<BroadcastMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: BroadcastMessage, ctx: &mut Self::Context) {
        // Send the broadcast message to each client
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            if text.contains("twitch") && text.contains("clip") {
                let url = text.clone();
               

                self.state.lock().unwrap().push_back(url.to_string().clone());
                ctx.text(format!("New clip added: {}", url));
            }
        }
    }
}

#[get("/ws/monitor")]
async fn ws_monitor(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppState>
) -> Result<HttpResponse, actix_web::Error> {
    let ws_session = WsSession {
        state: data.url_storage.clone(),
        ws_clients: data.ws_clients.clone(), // Pass ws_clients reference here
    };
    ws::start(ws_session, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        url_storage: Arc::new(Mutex::new(VecDeque::new())),
        ws_clients: Arc::new(Mutex::new(Vec::new())),
    });
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .service(oauth_callback)
            .service(start_capture)
            .service(ws_monitor)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
