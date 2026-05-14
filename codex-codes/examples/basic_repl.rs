//! Interactive REPL for the Codex CLI using the async app-server client.
//!
//! Maintains a single persistent thread across the session.
//! Type your prompt and press Enter. Type "exit" to quit.

use codex_codes::{
    AsyncClient, CommandApprovalDecision, CommandExecutionApprovalResponse,
    FileChangeApprovalDecision, FileChangeApprovalResponse, Notification, ServerMessage,
    ServerRequest, ThreadItem, ThreadStartParams, TurnStartParams, UserInput,
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("\nCodex REPL (app-server)");
    println!("======================");
    println!("Type your queries and press Enter. Type 'exit' to quit.\n");

    let mut client = AsyncClient::start().await?;
    let thread = client.thread_start(&ThreadStartParams::default()).await?;
    let thread_id = thread.thread_id().to_string();
    println!("Thread: {}\n", thread_id);

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Start a new turn with the user's input
        client
            .turn_start(&TurnStartParams {
                thread_id: thread_id.clone(),
                input: vec![UserInput::Text {
                    text: input.to_string(),
                }],
                model: None,
                reasoning_effort: None,
                sandbox_policy: None,
            })
            .await?;

        println!("\n--- Response ---");

        // Stream events until the turn completes
        loop {
            let msg = match client.next_message().await? {
                Some(m) => m,
                None => {
                    eprintln!("[connection closed]");
                    return Ok(());
                }
            };

            match msg {
                ServerMessage::Notification(n) => match n {
                    Notification::AgentMessageDelta(d) => {
                        print!("{}", d.delta);
                        io::stdout().flush()?;
                    }
                    Notification::CmdOutputDelta(d) => {
                        print!("{}", d.delta);
                        io::stdout().flush()?;
                    }
                    Notification::ReasoningDelta(d) => {
                        print!("[thinking] {}", d.delta);
                    }
                    Notification::ItemStarted(item_event) => match item_event.item {
                        ThreadItem::CommandExecution(c) => {
                            println!("\n[Command: {}]", c.command);
                        }
                        ThreadItem::FileChange(_) => {
                            println!("\n[File change]");
                        }
                        _ => {}
                    },
                    Notification::TurnCompleted(_) => {
                        println!();
                        break;
                    }
                    Notification::Error(e) => {
                        eprintln!("\n[Error: {}]", e.error);
                    }
                    other => {
                        log::debug!("Notification: {}", other.method());
                    }
                },
                ServerMessage::Request { id, request } => match request {
                    ServerRequest::CmdExecApproval(p) => {
                        println!("\n[Approving command: {}]", p.command);
                        client
                            .respond(
                                id,
                                &CommandExecutionApprovalResponse {
                                    decision: CommandApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    ServerRequest::FileChangeApproval(_) => {
                        println!("\n[Approving file change]");
                        client
                            .respond(
                                id,
                                &FileChangeApprovalResponse {
                                    decision: FileChangeApprovalDecision::Accept,
                                },
                            )
                            .await?;
                    }
                    ServerRequest::Unknown { method, .. } => {
                        eprintln!("[unhandled server request: {}]", method);
                    }
                },
            }
        }

        println!("--- End ---\n");
    }

    client.shutdown().await?;
    Ok(())
}
