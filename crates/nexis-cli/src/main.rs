use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use colored::Colorize;
use futures::StreamExt;
use nexis_cli::{CliClient, CliError, RoomInfoResponse};
use nexis_runtime::{AIProvider, AnthropicProvider, GenerateRequest, OpenAIProvider, StreamChunk};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

const REPL_COMMANDS: &[&str] = &[
    "login",
    "logout",
    "create-room",
    "join-room",
    "send",
    "reply",
    "invite-member",
    "list-rooms",
    "list-members",
    "search",
    "help",
    "@ai",
    "exit",
    "quit",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplCommand {
    Login(String),
    Logout,
    CreateRoom(String),
    JoinRoom(String),
    Send(String),
    Reply(String, String),
    InviteMember(String, String),
    ListRooms,
    ListMembers,
    Search(String),
    Help,
    Ai(String),
    Exit,
    Empty,
    Unknown(String),
}

#[derive(Default)]
struct ReplHelper;

impl Helper for ReplHelper {}
impl Hinter for ReplHelper {
    type Hint = String;
}
impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let pos = pos.min(line.len());
        let input = &line[..pos];
        let start = input
            .rfind(char::is_whitespace)
            .map_or(0, |last_ws| last_ws + 1);
        let prefix = &input[start..];

        let pairs = complete_candidates(prefix)
            .into_iter()
            .map(|candidate| Pair {
                display: candidate.to_string(),
                replacement: candidate.to_string(),
            })
            .collect();
        Ok((start, pairs))
    }
}

fn parse_command(line: &str) -> ReplCommand {
    let line = line.trim();
    if line.is_empty() {
        return ReplCommand::Empty;
    }

    if line == "exit" || line == "quit" {
        return ReplCommand::Exit;
    }
    if line == "help" || line == "?" {
        return ReplCommand::Help;
    }
    if line == "logout" {
        return ReplCommand::Logout;
    }
    if line == "list-rooms" {
        return ReplCommand::ListRooms;
    }
    if line == "list-members" {
        return ReplCommand::ListMembers;
    }
    if let Some(message) = line.strip_prefix("@ai ") {
        let message = message.trim();
        return if message.is_empty() {
            ReplCommand::Unknown("usage: @ai <message>".to_string())
        } else {
            ReplCommand::Ai(message.to_string())
        };
    }
    if line == "@ai" {
        return ReplCommand::Unknown("usage: @ai <message>".to_string());
    }

    let mut parts = line.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default();
    let tail = parts.next().map(str::trim).unwrap_or_default();

    match command {
        "login" if !tail.is_empty() => ReplCommand::Login(tail.to_string()),
        "login" => ReplCommand::Unknown("usage: login <member_id>".to_string()),
        "create-room" if !tail.is_empty() => ReplCommand::CreateRoom(tail.to_string()),
        "create-room" => ReplCommand::Unknown("usage: create-room <name>".to_string()),
        "join-room" if !tail.is_empty() => ReplCommand::JoinRoom(tail.to_string()),
        "join-room" => ReplCommand::Unknown("usage: join-room <room_id>".to_string()),
        "send" if !tail.is_empty() => ReplCommand::Send(tail.to_string()),
        "send" => ReplCommand::Unknown("usage: send <message>".to_string()),
        "search" if !tail.is_empty() => ReplCommand::Search(tail.to_string()),
        "search" => ReplCommand::Unknown("usage: search <query>".to_string()),
        "reply" => {
            let mut parts = tail.splitn(2, char::is_whitespace);
            let message_id = parts.next().unwrap_or_default();
            let message = parts.next().map(str::trim).unwrap_or_default();
            if message_id.is_empty() || message.is_empty() {
                ReplCommand::Unknown("usage: reply <message_id> <message>".to_string())
            } else {
                ReplCommand::Reply(message_id.to_string(), message.to_string())
            }
        }
        "invite-member" => {
            let mut parts = tail.splitn(2, char::is_whitespace);
            let room_id = parts.next().unwrap_or_default();
            let member_id = parts.next().map(str::trim).unwrap_or_default();
            if room_id.is_empty() || member_id.is_empty() {
                ReplCommand::Unknown("usage: invite-member <room_id> <member_id>".to_string())
            } else {
                ReplCommand::InviteMember(room_id.to_string(), member_id.to_string())
            }
        }
        _ => ReplCommand::Unknown(format!("unknown command: {line}")),
    }
}

fn help_text() -> String {
    [
        "Commands:",
        "  login <member_id>      Login as a member",
        "  logout                 Logout current member",
        "  create-room <name>     Create a room",
        "  join-room <room_id>    Join existing room",
        "  send <message>         Send message to current room",
        "  reply <message_id> <message>  Reply to a message",
        "  invite-member <room_id> <member_id>  Invite member to room",
        "  list-rooms             List known rooms",
        "  list-members           List members in current room",
        "  search <query>         Semantic search for messages",
        "  @ai <message>          Ask AI and stream response",
        "  help                   Show this help",
        "  exit | quit            Exit REPL",
    ]
    .join("\n")
}

#[derive(Debug)]
struct ReplState {
    member_id: Option<String>,
    current_room: Option<String>,
    known_rooms: BTreeMap<String, String>,
    client: CliClient,
}

impl ReplState {
    fn new(server: String) -> Self {
        Self {
            member_id: None,
            current_room: None,
            known_rooms: BTreeMap::new(),
            client: CliClient::new(server),
        }
    }
}

#[tokio::main]
async fn main() {
    if std::env::args().count() > 1 {
        let cli = nexis_cli::Cli::parse();
        match nexis_cli::run(cli).await {
            Ok(output) => {
                println!("{output}");
            }
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    let mut editor = match Editor::<ReplHelper, rustyline::history::DefaultHistory>::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("error: failed to start REPL: {err}");
            std::process::exit(1);
        }
    };
    editor.set_helper(Some(ReplHelper));
    let history = history_file();
    let _ = editor.load_history(&history);

    let server =
        std::env::var("NEXIS_SERVER").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let mut state = ReplState::new(server);
    println!(
        "{}",
        "Nexis CLI interactive mode. Type `help`.".bright_green()
    );
    loop {
        match editor.readline("nexis> ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = editor.add_history_entry(trimmed);
                match run_repl_command(&mut state, parse_command(trimmed)).await {
                    Ok(should_exit) => {
                        if should_exit {
                            break;
                        }
                    }
                    Err(err) => eprintln!("{} {}", "error:".red(), err),
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted)
            | Err(rustyline::error::ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(err) => {
                eprintln!("{} {err}", "error:".red());
                break;
            }
        }
    }

    if let Err(err) = editor.save_history(&history) {
        eprintln!("{} failed to save history: {err}", "warning:".yellow());
    }
}

async fn run_repl_command(state: &mut ReplState, command: ReplCommand) -> Result<bool, CliError> {
    match command {
        ReplCommand::Login(member_id) => {
            state.member_id = Some(member_id.clone());
            println!("{} {}", "logged in as".green(), member_id.cyan());
        }
        ReplCommand::Logout => {
            state.member_id = None;
            println!("{}", "logged out".green());
        }
        ReplCommand::CreateRoom(name) => {
            let created = state.client.create_room(name, None).await?;
            state
                .known_rooms
                .insert(created.id.clone(), created.name.clone());
            state.current_room = Some(created.id.clone());
            println!(
                "{} {} ({})",
                "room created:".green(),
                created.id.cyan(),
                created.name
            );
        }
        ReplCommand::JoinRoom(room_id) => {
            let room = state.client.get_room(&room_id).await?;
            state.known_rooms.insert(room.id.clone(), room.name.clone());
            state.current_room = Some(room.id.clone());
            println!("{} {}", "joined room".green(), room.id.cyan());
        }
        ReplCommand::Send(message) => {
            let member_id = state.member_id.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("login required before `send`".to_string())
            })?;
            let room_id = state.current_room.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("join-room required before `send`".to_string())
            })?;
            let sent = state
                .client
                .send_message(room_id.to_string(), member_id.to_string(), message)
                .await?;
            println!("{} {}", "message sent:".green(), sent.id.cyan());
        }
        ReplCommand::Reply(message_id, message) => {
            let member_id = state.member_id.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("login required before `reply`".to_string())
            })?;
            let room_id = state.current_room.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("join-room required before `reply`".to_string())
            })?;
            let sent = state
                .client
                .reply_message(room_id.to_string(), member_id.to_string(), message_id.clone(), message)
                .await?;
            println!("{} {} (reply to {})", "message sent:".green(), sent.id.cyan(), message_id);
        }
        ReplCommand::InviteMember(room_id, member_id) => {
            let _ = state.member_id.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("login required before `invite-member`".to_string())
            })?;
            let _result = state.client.invite_member(&room_id, &member_id).await?;
            println!("{} {} to room {}", "invited".green(), member_id.cyan(), room_id.cyan());
        }
        ReplCommand::ListRooms => {
            if state.known_rooms.is_empty() {
                println!("{}", "no known rooms yet".yellow());
            } else {
                for (room_id, room_name) in &state.known_rooms {
                    let marker = if state.current_room.as_deref() == Some(room_id.as_str()) {
                        "*"
                    } else {
                        " "
                    };
                    println!("{marker} {room_id} ({room_name})");
                }
            }
        }
        ReplCommand::ListMembers => {
            let room_id = state.current_room.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("join-room required before `list-members`".to_string())
            })?;
            let room = state.client.get_room(room_id).await?;
            print_members(&room, state.member_id.as_deref());
        }
        ReplCommand::Search(query) => {
            let room_id = state.current_room.as_ref().and_then(|r| r.parse::<uuid::Uuid>().ok());
            let response = state.client.search(&query, 10, room_id, None).await?;
            println!("{}", format!("Search results for: {}", response.query).bright_blue());
            if response.results.is_empty() {
                println!("{}", "No results found.".yellow());
            } else {
                for (i, result) in response.results.iter().enumerate() {
                    println!(
                        "{}. {} [score: {:.3}]",
                        (i + 1).to_string().cyan(),
                        result.content.chars().take(80).collect::<String>(),
                        result.score
                    );
                    if let Some(room_id) = result.room_id {
                        println!("   {}", format!("Room: {}", room_id).dimmed());
                    }
                }
                println!("{}", format!("Total: {} results", response.total).green());
            }
        }
        ReplCommand::Help => {
            println!("{}", help_text().bright_blue());
        }
        ReplCommand::Ai(prompt) => {
            let room_id = state.current_room.as_deref().ok_or_else(|| {
                CliError::InvalidArgument("join-room required before `@ai`".to_string())
            })?;
            let reply = stream_ai_response(&prompt).await?;
            let ai_sender = std::env::var("NEXIS_AI_MEMBER")
                .unwrap_or_else(|_| "nexis:ai:assistant".to_string());
            let _ = state
                .client
                .send_message(room_id.to_string(), ai_sender, reply)
                .await?;
        }
        ReplCommand::Exit => {
            println!("{}", "bye".bright_green());
            return Ok(true);
        }
        ReplCommand::Empty => {}
        ReplCommand::Unknown(message) => {
            println!("{} {message}", "warning:".yellow());
            println!("{}", "Type `help` for available commands.".yellow());
        }
    }

    Ok(false)
}

fn print_members(room: &RoomInfoResponse, current_member: Option<&str>) {
    let mut members = BTreeSet::new();
    if let Some(member) = current_member {
        members.insert(member.to_string());
    }
    for msg in &room.messages {
        members.insert(msg.sender.clone());
    }

    if members.is_empty() {
        println!("{}", "no members in room".yellow());
        return;
    }

    for member in members {
        println!("- {member}");
    }
}

async fn stream_ai_response(prompt: &str) -> Result<String, CliError> {
    let provider_name = std::env::var("NEXIS_AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let provider: Arc<dyn AIProvider> = match provider_name.as_str() {
        "openai" => Arc::new(OpenAIProvider::from_env()),
        "anthropic" => Arc::new(AnthropicProvider::from_env()),
        other => {
            return Err(CliError::InvalidArgument(format!(
                "unsupported AI provider `{other}`"
            )));
        }
    };

    let request = GenerateRequest {
        prompt: prompt.to_string(),
        model: std::env::var("NEXIS_AI_MODEL").ok(),
        max_tokens: Some(300),
        temperature: Some(0.7),
        metadata: None,
    };

    let mut stream = provider
        .generate_stream(request)
        .await
        .map_err(|err| CliError::HttpTransport(err.to_string()))?;

    println!("{}", "AI:".bright_magenta());
    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk.map_err(|err| CliError::HttpTransport(err.to_string()))? {
            StreamChunk::Delta { text } => {
                response.push_str(&text);
                print!("{text}");
                let _ = io::stdout().flush();
            }
            StreamChunk::Done => {}
        }
    }
    println!();
    Ok(response)
}

fn history_file() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".nexis-cli-history");
    }
    PathBuf::from(".nexis-cli-history")
}

fn complete_candidates(prefix: &str) -> BTreeSet<&'static str> {
    REPL_COMMANDS
        .iter()
        .copied()
        .filter(|command| command.starts_with(prefix))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{complete_candidates, help_text, parse_command, ReplCommand};

    #[test]
    fn parse_send_uses_message_tail() {
        let command = parse_command("send hello world");
        assert_eq!(command, ReplCommand::Send("hello world".to_string()));
    }

    #[test]
    fn parse_ai_command() {
        let command = parse_command("@ai summarize this");
        assert_eq!(command, ReplCommand::Ai("summarize this".to_string()));
    }

    #[test]
    fn parse_login_requires_member_id() {
        let command = parse_command("login");
        assert_eq!(
            command,
            ReplCommand::Unknown("usage: login <member_id>".to_string())
        );
    }

    #[test]
    fn complete_candidates_matches_prefix() {
        let lo_candidates = complete_candidates("lo");
        assert!(lo_candidates.contains("login"));
        assert!(lo_candidates.contains("logout"));
        
        let li_candidates = complete_candidates("li");
        assert!(li_candidates.contains("list-rooms"));
        assert!(li_candidates.contains("list-members"));
    }

    #[test]
    fn help_text_lists_core_commands() {
        let help = help_text();
        for command in [
            "login <member_id>",
            "logout",
            "create-room <name>",
            "join-room <room_id>",
            "send <message>",
            "reply <message_id>",
            "invite-member <room_id>",
            "list-rooms",
            "list-members",
            "@ai <message>",
        ] {
            assert!(help.contains(command), "help text missing `{command}`");
        }
    }
}
