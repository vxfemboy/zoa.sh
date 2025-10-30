use crate::mods::shoutbox_protocol::ShoutboxCommand as ProtoCommand;
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub struct ShoutboxMessage {
    pub id: String,
    pub username: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoutboxCommand {
    pub action: String,
    pub username: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoutboxResponse {
    pub action: String,
    pub messages: Option<Vec<ShoutboxMessage>>,
    pub message: Option<ShoutboxMessage>,
    pub error: Option<String>,
}

// WebSocket actor for handling real-time communication
pub struct ShoutboxSession {
    pub id: String,
    pub addr: Addr<ShoutboxServer>,
}

impl Actor for ShoutboxSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Register this session with the server
        let addr = ctx.address();
        let server_addr = self.addr.clone();
        let session_id = self.id.clone();
        actix::spawn(async move {
            let _ = server_addr
                .send(Connect {
                    id: session_id,
                    addr,
                })
                .await;
        });
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        // Unregister this session from the server
        let server_addr = self.addr.clone();
        let session_id = self.id.clone();
        actix::spawn(async move {
            let _ = server_addr.send(Disconnect { id: session_id }).await;
        });
    }
}

impl Handler<ShoutboxMessage> for ShoutboxSession {
    type Result = ();

    fn handle(&mut self, msg: ShoutboxMessage, ctx: &mut Self::Context) {
        // Support legacy action-based messages for current frontend
        if msg.username.is_empty() && msg.content.starts_with('{') {
            // Forward JSON (e.g., user_count)
            ctx.text(msg.content);
        } else {
            let response = ShoutboxResponse {
                action: "new_message".to_string(),
                messages: None,
                message: Some(msg),
                error: None,
            };
            ctx.text(serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()));
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ShoutboxSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // Try unified protocol first, then fallback to legacy action-based
                if let Ok(command) = serde_json::from_str::<ProtoCommand>(&text) {
                    match command {
                        ProtoCommand::GetMessages => {
                            let addr = self.addr.clone();
                            let session_addr = ctx.address();
                            actix::spawn(async move {
                                if let Ok(messages) = addr.send(GetMessages).await {
                                    let response = ShoutboxResponse {
                                        action: "messages".to_string(),
                                        messages: Some(messages),
                                        message: None,
                                        error: None,
                                    };
                                    let json = serde_json::to_string(&response)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    session_addr.do_send(ShoutboxMessage {
                                        id: String::new(),
                                        username: String::new(),
                                        content: json,
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            });
                        }
                        ProtoCommand::SendMessage { username, content } => {
                            if !username.trim().is_empty() && !content.trim().is_empty() {
                                let message = ShoutboxMessage {
                                    id: Uuid::new_v4().to_string(),
                                    username: username.trim().to_string(),
                                    content: content.trim().to_string(),
                                    timestamp: chrono::Utc::now(),
                                };

                                let addr = self.addr.clone();
                                actix::spawn(async move {
                                    let _ = addr.send(message).await;
                                });
                            }
                        }
                    }
                } else if let Ok(command) = serde_json::from_str::<ShoutboxCommand>(&text) {
                    match command.action.as_str() {
                        "get_messages" => {
                            let addr = self.addr.clone();
                            let session_addr = ctx.address();
                            actix::spawn(async move {
                                if let Ok(messages) = addr.send(GetMessages).await {
                                    let response = ShoutboxResponse {
                                        action: "messages".to_string(),
                                        messages: Some(messages),
                                        message: None,
                                        error: None,
                                    };
                                    let json = serde_json::to_string(&response)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    session_addr.do_send(ShoutboxMessage {
                                        id: String::new(),
                                        username: String::new(),
                                        content: json,
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            });
                        }
                        "send_message" => {
                            if let (Some(username), Some(content)) =
                                (command.username, command.content)
                            {
                                if !username.trim().is_empty() && !content.trim().is_empty() {
                                    let message = ShoutboxMessage {
                                        id: Uuid::new_v4().to_string(),
                                        username: username.trim().to_string(),
                                        content: content.trim().to_string(),
                                        timestamp: chrono::Utc::now(),
                                    };
                                    let addr = self.addr.clone();
                                    actix::spawn(async move {
                                        let _ = addr.send(message).await;
                                    });
                                }
                            }
                        }
                        _ => {
                            let response = ShoutboxResponse {
                                action: "error".to_string(),
                                messages: None,
                                message: None,
                                error: Some("Unknown command".to_string()),
                            };
                            ctx.text(
                                serde_json::to_string(&response)
                                    .unwrap_or_else(|_| "{}".to_string()),
                            );
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

// Server actor to manage all sessions and messages
#[derive(Default)]
pub struct ShoutboxServer {
    sessions: HashMap<String, Addr<ShoutboxSession>>,
    messages: Vec<ShoutboxMessage>,
}

impl Actor for ShoutboxServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ShoutboxServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.id, msg.addr);
        self.broadcast_user_count();
    }
}

impl Handler<Disconnect> for ShoutboxServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
        self.broadcast_user_count();
    }
}

impl Handler<ShoutboxMessage> for ShoutboxServer {
    type Result = ();

    fn handle(&mut self, msg: ShoutboxMessage, _: &mut Context<Self>) {
        // Store the message
        self.messages.push(msg.clone());

        // Keep only the last 50 messages to prevent memory issues
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }

        // Broadcast to all connected sessions
        for session in self.sessions.values() {
            session.do_send(msg.clone());
        }
    }
}

impl Handler<GetMessages> for ShoutboxServer {
    type Result = Vec<ShoutboxMessage>;

    fn handle(&mut self, _: GetMessages, _: &mut Context<Self>) -> Self::Result {
        self.messages.clone()
    }
}

impl ShoutboxServer {
    fn broadcast_user_count(&self) {
        let user_count = self.sessions.len();
        let user_count_response = serde_json::json!({
            "action": "user_count",
            "user_count": user_count
        });

        for session in self.sessions.values() {
            // Send user count update to each session
            session.do_send(ShoutboxMessage {
                id: "".to_string(),
                username: "".to_string(),
                content: user_count_response.to_string(),
                timestamp: chrono::Utc::now(),
            });
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub id: String,
    pub addr: Addr<ShoutboxSession>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: String,
}

#[derive(Message)]
#[rtype(result = "Vec<ShoutboxMessage>")]
pub struct GetMessages;

// WebSocket handler
pub async fn shoutbox_ws(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<ShoutboxServer>>,
) -> Result<HttpResponse, Error> {
    let resp = ws::start(
        ShoutboxSession {
            id: Uuid::new_v4().to_string(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

// HTTP API endpoints for shoutbox
pub async fn get_shoutbox_messages(
    srv: web::Data<Addr<ShoutboxServer>>,
) -> Result<HttpResponse, Error> {
    let messages = srv
        .send(GetMessages)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to get messages"))?;

    Ok(HttpResponse::Ok().json(messages))
}

pub async fn post_shoutbox_message(
    srv: web::Data<Addr<ShoutboxServer>>,
    message: web::Json<ShoutboxMessage>,
) -> Result<HttpResponse, Error> {
    let message = message.into_inner();
    srv.send(message)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to send message"))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "success"
    })))
}
