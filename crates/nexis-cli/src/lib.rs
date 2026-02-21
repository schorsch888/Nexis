use std::time::Duration;

use clap::{Parser, Subcommand};
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub fn crate_name() -> &'static str {
    "nexis-cli"
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "nexis-cli",
    version,
    about = "Nexis command line client",
    long_about = "Nexis command line client for creating rooms, sending messages, and connecting over WebSocket"
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        default_value = "http://127.0.0.1:8080",
        help = "Control Plane base HTTP URL"
    )]
    pub server: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    #[command(about = "Create a room")]
    CreateRoom {
        #[arg(help = "Room name")]
        name: String,
        #[arg(long, help = "Optional room topic")]
        topic: Option<String>,
    },
    #[command(about = "Send a text message to a room")]
    SendMessage {
        #[arg(help = "Room ID")]
        room_id: String,
        #[arg(help = "Sender member ID, e.g. nexis:human:alice@example.com")]
        sender: String,
        #[arg(help = "Message body")]
        text: String,
    },
    #[command(about = "Connect to WebSocket endpoint")]
    Connect {
        #[arg(long, default_value = "ws://127.0.0.1:8080/ws", help = "WebSocket URL")]
        url: String,
        #[arg(long, help = "Optional text frame to send immediately after connect")]
        message: Option<String>,
        #[arg(
            long,
            default_value_t = 5_000,
            help = "Receive timeout in milliseconds"
        )]
        timeout_ms: u64,
    },
    #[command(about = "Test AI provider connection")]
    TestProvider {
        #[arg(short, long, help = "Provider to test (openai or anthropic)")]
        provider: String,
        #[arg(short, long, help = "Prompt to send")]
        prompt: String,
        #[arg(short, long, help = "Use streaming")]
        stream: bool,
    },
    #[command(about = "Semantic search for messages")]
    Search {
        #[arg(help = "Search query")]
        query: String,
        #[arg(long, default_value_t = 10, help = "Maximum number of results")]
        limit: usize,
        #[arg(long, help = "Filter by room ID")]
        room: Option<String>,
        #[arg(long, help = "Minimum similarity score (0.0-1.0)")]
        min_score: Option<f32>,
    },
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("http transport error: {0}")]
    HttpTransport(String),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("json decode error: {0}")]
    Decode(String),
    #[error("websocket error: {0}")]
    WebSocket(String),
    #[error("timeout waiting for websocket frame after {timeout_ms}ms")]
    WebSocketTimeout { timeout_ms: u64 },
    #[error("connection closed before receiving a websocket frame")]
    WebSocketClosed,
}

#[derive(Debug, Clone)]
pub struct CliClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
struct CreateRoomRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRoomResponse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
struct SendMessageRequest {
    #[serde(rename = "roomId")]
    room_id: String,
    sender: String,
    text: String,
    #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageResponse {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub sender: String,
    pub text: String,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomInfoResponse {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    #[serde(default)]
    pub messages: Vec<StoredMessage>,
}

#[derive(Debug, Clone, Serialize)]
struct InviteMemberRequest {
    #[serde(rename = "memberId")]
    member_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InviteMemberResponse {
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "memberId")]
    pub member_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct SearchRequest {
    query: String,
    limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResultItem>,
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResultItem {
    pub id: uuid::Uuid,
    pub score: f32,
    pub content: String,
    pub room_id: Option<uuid::Uuid>,
}

impl CliClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("reqwest client should build"),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn create_room(
        &self,
        name: String,
        topic: Option<String>,
    ) -> Result<CreateRoomResponse, CliError> {
        if name.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "room name cannot be empty".to_string(),
            ));
        }

        let payload = CreateRoomRequest { name, topic };
        self.post_json("/v1/rooms", &payload).await
    }

    pub async fn send_message(
        &self,
        room_id: String,
        sender: String,
        text: String,
    ) -> Result<SendMessageResponse, CliError> {
        self.send_message_with_reply(room_id, sender, text, None).await
    }

    pub async fn send_message_with_reply(
        &self,
        room_id: String,
        sender: String,
        text: String,
        reply_to: Option<String>,
    ) -> Result<SendMessageResponse, CliError> {
        if room_id.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "room id cannot be empty".to_string(),
            ));
        }
        if sender.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "sender cannot be empty".to_string(),
            ));
        }
        if text.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "message text cannot be empty".to_string(),
            ));
        }

        let payload = SendMessageRequest {
            room_id,
            sender,
            text,
            reply_to,
        };
        self.post_json("/v1/messages", &payload).await
    }

    pub async fn reply_message(
        &self,
        room_id: String,
        sender: String,
        reply_to: String,
        text: String,
    ) -> Result<SendMessageResponse, CliError> {
        if reply_to.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "reply_to message id cannot be empty".to_string(),
            ));
        }
        self.send_message_with_reply(room_id, sender, text, Some(reply_to)).await
    }

    pub async fn get_room(&self, room_id: &str) -> Result<RoomInfoResponse, CliError> {
        if room_id.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "room id cannot be empty".to_string(),
            ));
        }
        self.get_json(&format!("/v1/rooms/{room_id}")).await
    }

    pub async fn invite_member(
        &self,
        room_id: &str,
        member_id: &str,
    ) -> Result<InviteMemberResponse, CliError> {
        if room_id.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "room id cannot be empty".to_string(),
            ));
        }
        if member_id.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "member id cannot be empty".to_string(),
            ));
        }
        let payload = InviteMemberRequest {
            member_id: member_id.to_string(),
        };
        self.post_json(&format!("/v1/rooms/{room_id}/invite"), &payload).await
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        room_id: Option<uuid::Uuid>,
        min_score: Option<f32>,
    ) -> Result<SearchResponse, CliError> {
        if query.trim().is_empty() {
            return Err(CliError::InvalidArgument(
                "query cannot be empty".to_string(),
            ));
        }
        let payload = SearchRequest {
            query: query.to_string(),
            limit,
            min_score,
            room_id,
        };
        self.post_json("/v1/search", &payload).await
    }

    async fn post_json<TReq, TRes>(&self, path: &str, payload: &TReq) -> Result<TRes, CliError>
    where
        TReq: Serialize + Sync,
        TRes: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .post(self.endpoint(path))
            .json(payload)
            .send()
            .await
            .map_err(|err| CliError::HttpTransport(err.to_string()))?;

        if response.status() != StatusCode::OK && response.status() != StatusCode::CREATED {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unable to read body>".to_string());
            return Err(CliError::HttpStatus { status, body });
        }

        response
            .json::<TRes>()
            .await
            .map_err(|err| CliError::Decode(err.to_string()))
    }

    async fn get_json<TRes>(&self, path: &str) -> Result<TRes, CliError>
    where
        TRes: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .get(self.endpoint(path))
            .send()
            .await
            .map_err(|err| CliError::HttpTransport(err.to_string()))?;

        if response.status() != StatusCode::OK {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unable to read body>".to_string());
            return Err(CliError::HttpStatus { status, body });
        }

        response
            .json::<TRes>()
            .await
            .map_err(|err| CliError::Decode(err.to_string()))
    }
}

pub async fn connect_websocket_once(
    url: &str,
    message: Option<String>,
    timeout_ms: u64,
) -> Result<Option<String>, CliError> {
    let (mut ws, _) = connect_async(url)
        .await
        .map_err(|err| CliError::WebSocket(err.to_string()))?;

    if let Some(text) = message {
        ws.send(Message::Text(text))
            .await
            .map_err(|err| CliError::WebSocket(err.to_string()))?;

        let maybe_msg = timeout(Duration::from_millis(timeout_ms), ws.next())
            .await
            .map_err(|_| CliError::WebSocketTimeout { timeout_ms })?;

        match maybe_msg {
            None => Err(CliError::WebSocketClosed),
            Some(Ok(Message::Text(text))) => Ok(Some(text.to_string())),
            Some(Ok(Message::Binary(_))) => Ok(Some("<binary frame>".to_string())),
            Some(Ok(Message::Close(_))) => Err(CliError::WebSocketClosed),
            Some(Ok(_)) => Ok(None),
            Some(Err(err)) => Err(CliError::WebSocket(err.to_string())),
        }
    } else {
        Ok(None)
    }
}

pub async fn run(cli: Cli) -> Result<String, CliError> {
    match cli.command {
        Commands::CreateRoom { name, topic } => {
            let client = CliClient::new(cli.server);
            let created = client.create_room(name, topic).await?;
            Ok(format!("room created: {} ({})", created.id, created.name))
        }
        Commands::SendMessage {
            room_id,
            sender,
            text,
        } => {
            let client = CliClient::new(cli.server);
            let sent = client.send_message(room_id, sender, text).await?;
            Ok(format!("message sent: {}", sent.id))
        }
        Commands::Connect {
            url,
            message,
            timeout_ms,
        } => {
            let reply = connect_websocket_once(&url, message, timeout_ms).await?;
            match reply {
                Some(text) => Ok(format!("ws reply: {text}")),
                None => Ok("ws connected".to_string()),
            }
        }
        Commands::TestProvider {
            provider,
            prompt,
            stream,
        } => test_provider(&provider, &prompt, stream).await,
        Commands::Search {
            query,
            limit,
            room,
            min_score,
        } => {
            let client = CliClient::new(cli.server);
            let room_id = room.and_then(|r| r.parse::<uuid::Uuid>().ok());
            let response = client.search(&query, limit, room_id, min_score).await?;
            let mut output = format!("Search results for: {}\n\n", response.query);
            if response.results.is_empty() {
                output.push_str("No results found.\n");
            } else {
                for (i, result) in response.results.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. [score: {:.3}] {}\n",
                        i + 1,
                        result.score,
                        result.content.chars().take(100).collect::<String>()
                    ));
                    if let Some(room_id) = result.room_id {
                        output.push_str(&format!("   Room: {}\n", room_id));
                    }
                    output.push('\n');
                }
                output.push_str(&format!("Total: {} results\n", response.total));
            }
            Ok(output)
        }
    }
}

async fn test_provider(provider: &str, prompt: &str, stream: bool) -> Result<String, CliError> {
    use nexis_runtime::{AIProvider, AnthropicProvider, GenerateRequest, OpenAIProvider};
    use std::sync::Arc;

    println!("Testing {} provider...", provider);

    let provider: Arc<dyn AIProvider> = match provider {
        "openai" => Arc::new(OpenAIProvider::from_env()),
        "anthropic" => Arc::new(AnthropicProvider::from_env()),
        _ => {
            return Err(CliError::InvalidArgument(format!(
                "Unknown provider: {}",
                provider
            )))
        }
    };

    let req = GenerateRequest {
        prompt: prompt.to_string(),
        model: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        metadata: None,
    };

    if stream {
        println!("Streaming response:\n");
        use futures::StreamExt;
        let mut stream = provider
            .generate_stream(req)
            .await
            .map_err(|e| CliError::HttpTransport(e.to_string()))?;

        while let Some(chunk) = stream.next().await {
            match chunk.map_err(|e| CliError::HttpTransport(e.to_string()))? {
                nexis_runtime::StreamChunk::Delta { text } => print!("{}", text),
                nexis_runtime::StreamChunk::Done => println!(),
            }
        }
        Ok("Stream completed".to_string())
    } else {
        println!("Sending request...\n");
        let resp = provider
            .generate(req)
            .await
            .map_err(|e| CliError::HttpTransport(e.to_string()))?;
        println!("Response: {}", resp.content);
        println!("Model: {:?}", resp.model);
        println!("Finish reason: {:?}", resp.finish_reason);
        Ok(format!("Response: {}", resp.content))
    }
}

#[cfg(test)]
mod tests {
    use super::{connect_websocket_once, Cli, CliClient, CliError, Commands};
    use clap::Parser;
    use futures::{SinkExt, StreamExt};
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    fn network_tests_enabled() -> bool {
        matches!(std::env::var("NEXIS_RUN_NETWORK_TESTS"), Ok(value) if value == "1")
    }

    #[test]
    fn cli_parses_create_room_command() {
        let cli = Cli::parse_from(["nexis-cli", "create-room", "general", "--topic", "team"]);
        match cli.command {
            Commands::CreateRoom { name, topic } => {
                assert_eq!(name, "general");
                assert_eq!(topic.as_deref(), Some("team"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn cli_parses_send_message_command() {
        let cli = Cli::parse_from([
            "nexis-cli",
            "send-message",
            "room_1",
            "nexis:human:alice@example.com",
            "hello",
        ]);

        match cli.command {
            Commands::SendMessage {
                room_id,
                sender,
                text,
            } => {
                assert_eq!(room_id, "room_1");
                assert_eq!(sender, "nexis:human:alice@example.com");
                assert_eq!(text, "hello");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_room_rejects_empty_id() {
        let client = CliClient::new("http://127.0.0.1:8080");
        let error = client.get_room("").await.unwrap_err();
        match error {
            CliError::InvalidArgument(message) => {
                assert!(message.contains("room id cannot be empty"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_room_calls_control_plane_http_endpoint() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        let room_mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/rooms");
                then.status(201)
                    .json_body(json!({"id": "room_general", "name": "general"}));
            })
            .await;

        let client = CliClient::new(server.base_url());
        let room = client
            .create_room("general".to_string(), Some("team".to_string()))
            .await
            .unwrap();

        room_mock.assert_async().await;
        assert_eq!(room.id, "room_general");
        assert_eq!(room.name, "general");
    }

    #[tokio::test]
    async fn send_message_surfaces_http_status_error() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/messages");
                then.status(400).body("invalid sender");
            })
            .await;

        let client = CliClient::new(server.base_url());
        let error = client
            .send_message(
                "room_1".to_string(),
                "bad-sender".to_string(),
                "hello".to_string(),
            )
            .await
            .unwrap_err();

        match error {
            CliError::HttpStatus { status, body } => {
                assert_eq!(status, 400);
                assert!(body.contains("invalid sender"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn connect_command_receives_echo_text() {
        if !network_tests_enabled() {
            eprintln!("skipping network test: set NEXIS_RUN_NETWORK_TESTS=1 to enable");
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            if let Some(Ok(Message::Text(text))) = ws.next().await {
                ws.send(Message::Text(text)).await.unwrap();
            }
        });

        let response =
            connect_websocket_once(&format!("ws://{}/", addr), Some("ping".to_string()), 2_000)
                .await
                .unwrap();

        assert_eq!(response.as_deref(), Some("ping"));
    }
}
