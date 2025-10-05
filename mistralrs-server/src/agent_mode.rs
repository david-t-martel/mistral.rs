use either::Either;
use indexmap::IndexMap;
use mistralrs_agent_tools::AgentTools;
use mistralrs_core::{
    ChunkChoice, Constraint, Delta, MessageContent, MistralRs, NormalRequest, Request,
    RequestMessage, Response, SamplingParams, WebSearchOptions, TERMINATE_ALL_NEXT_STEP,
};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{
    io::{self, Write as IoWrite},
    sync::{atomic::Ordering, Arc},
    time::Instant,
};
use tokio::sync::mpsc::channel;
use tracing::{error, info};

use crate::interactive_mode::{history_file_path, CTRLC_HANDLER};

const AGENT_MODE_HELP: &str = r#"
Welcome to Agent Mode! This mode enables autonomous reasoning with automatic tool execution.

The model will:
- Reason about your query
- Automatically call and execute tools as needed
- Synthesize results into a coherent answer

All tool execution happens automatically within the inference engine.

Commands:
- `\help`: Display this message.
- `\exit`: Quit agent mode.
- `\clear`: Clear the chat history.
- `\temperature <float>`: Set sampling temperature (0.0 to 2.0).
- `\topk <int>`: Set top-k sampling value (>0).
- `\topp <float>`: Set top-p sampling value in (0.0 to 1.0).
"#;

const HELP_CMD: &str = "\\help";
const EXIT_CMD: &str = "\\exit";
const CLEAR_CMD: &str = "\\clear";
const TEMPERATURE_CMD: &str = "\\temperature";
const TOPK_CMD: &str = "\\topk";
const TOPP_CMD: &str = "\\topp";

fn exit_handler() {
    std::process::exit(0);
}

fn terminate_handler() {
    TERMINATE_ALL_NEXT_STEP.store(true, Ordering::SeqCst);
}

fn agent_sample_parameters() -> SamplingParams {
    SamplingParams {
        temperature: Some(0.7),
        top_k: Some(50),
        top_p: Some(0.9),
        min_p: Some(0.05),
        top_n_logprobs: 0,
        frequency_penalty: Some(0.0),
        presence_penalty: Some(0.0),
        repetition_penalty: None,
        max_len: None,
        stop_toks: None,
        logits_bias: None,
        n_choices: 1,
        dry_params: None,
    }
}

fn read_line(editor: &mut DefaultEditor) -> String {
    let r = editor.readline("> ");
    match r {
        Err(ReadlineError::Interrupted) => {
            editor.save_history(&history_file_path()).unwrap();
            std::process::exit(0);
        }
        Err(ReadlineError::Eof) => {
            editor.save_history(&history_file_path()).unwrap();
            std::process::exit(0);
        }
        Err(e) => {
            editor.save_history(&history_file_path()).unwrap();
            eprintln!("Error reading input: {e:?}");
            std::process::exit(1);
        }
        Ok(prompt) => {
            editor.add_history_entry(prompt.clone()).unwrap();
            prompt
        }
    }
}

fn handle_sampling_command(prompt: &str, sampling_params: &mut SamplingParams) -> bool {
    let trimmed = prompt.trim();
    if trimmed.starts_with(TEMPERATURE_CMD) {
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if let [_, value] = parts.as_slice() {
            match value.trim().parse::<f64>() {
                Ok(v) if v > 0.0 && v <= 2.0 => {
                    sampling_params.temperature = Some(v);
                    info!("Set temperature to {v}");
                }
                Ok(_) => {
                    println!("Error: temperature must be in (0.0, 2.0]");
                }
                Err(_) => println!("Error: format is `{TEMPERATURE_CMD} <float>`"),
            }
        } else {
            println!("Error: format is `{TEMPERATURE_CMD} <float>`");
        }
        return true;
    }
    if trimmed.starts_with(TOPK_CMD) {
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if let [_, value] = parts.as_slice() {
            match value.trim().parse::<usize>() {
                Ok(v) if v > 0 => {
                    sampling_params.top_k = Some(v);
                    info!("Set top-k to {v}");
                }
                Ok(_) => {
                    println!("Error: top-k must be a positive integer");
                }
                Err(_) => println!("Error: format is `{TOPK_CMD} <int>`"),
            }
        } else {
            println!("Error: format is `{TOPK_CMD} <int>`");
        }
        return true;
    }
    if trimmed.starts_with(TOPP_CMD) {
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        if let [_, value] = parts.as_slice() {
            match value.trim().parse::<f64>() {
                Ok(v) if v > 0.0 && v <= 1.0 => {
                    sampling_params.top_p = Some(v);
                    info!("Set top-p to {v}");
                }
                Ok(_) => {
                    println!("Error: top-p must be in (0.0, 1.0]");
                }
                Err(_) => println!("Error: format is `{TOPP_CMD} <float>`"),
            }
        } else {
            println!("Error: format is `{TOPP_CMD} <float>`");
        }
        return true;
    }
    false
}

/// Execute tool calls using the agent tools
fn execute_tool_calls(
    agent_tools: &AgentTools,
    tool_calls: &[mistralrs_core::ToolCallResponse],
) -> Vec<String> {
    let mut results = Vec::new();

    for tool_call in tool_calls {
        let function_name = &tool_call.function.name;
        let arguments = &tool_call.function.arguments;

        info!("Executing tool: {} with args: {}", function_name, arguments);

        // Parse arguments
        let result = match serde_json::from_str::<serde_json::Value>(arguments) {
            Ok(args) => {
                // Route to appropriate agent tool based on function name
                match function_name.as_str() {
                    "read_file" | "read" => {
                        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                            match agent_tools.read(path) {
                                Ok(fs_result) => serde_json::to_string(&fs_result)
                                    .unwrap_or_else(|_| "Error serializing result".to_string()),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'path' parameter".to_string()
                        }
                    }
                    "write_file" | "write" => {
                        let path = args.get("path").and_then(|v| v.as_str());
                        let content = args.get("content").and_then(|v| v.as_str());
                        let create = args.get("create").and_then(|v| v.as_bool()).unwrap_or(true);
                        let overwrite = args
                            .get("overwrite")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if let (Some(path), Some(content)) = (path, content) {
                            match agent_tools.write(path, content, create, overwrite) {
                                Ok(fs_result) => serde_json::to_string(&fs_result)
                                    .unwrap_or_else(|_| "Error serializing result".to_string()),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'path' or 'content' parameter".to_string()
                        }
                    }
                    "append_file" | "append" => {
                        let path = args.get("path").and_then(|v| v.as_str());
                        let content = args.get("content").and_then(|v| v.as_str());

                        if let (Some(path), Some(content)) = (path, content) {
                            match agent_tools.append(path, content) {
                                Ok(fs_result) => serde_json::to_string(&fs_result)
                                    .unwrap_or_else(|_| "Error serializing result".to_string()),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'path' or 'content' parameter".to_string()
                        }
                    }
                    "delete_file" | "delete" => {
                        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                            match agent_tools.delete(path) {
                                Ok(fs_result) => serde_json::to_string(&fs_result)
                                    .unwrap_or_else(|_| "Error serializing result".to_string()),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'path' parameter".to_string()
                        }
                    }
                    "find_files" | "find" => {
                        let pattern = args.get("pattern").and_then(|v| v.as_str());
                        let max_depth = args
                            .get("max_depth")
                            .and_then(|v| v.as_u64())
                            .map(|d| d as usize);

                        if let Some(pattern) = pattern {
                            match agent_tools.find(pattern, max_depth) {
                                Ok(files) => serde_json::to_string(&files)
                                    .unwrap_or_else(|_| "Error serializing result".to_string()),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'pattern' parameter".to_string()
                        }
                    }
                    "list_tree" | "tree" => {
                        let root = args
                            .get("root")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let max_depth = args
                            .get("max_depth")
                            .and_then(|v| v.as_u64())
                            .map(|d| d as usize);

                        match agent_tools.tree(root, max_depth) {
                            Ok(tree) => serde_json::to_string(&tree)
                                .unwrap_or_else(|_| "Error serializing result".to_string()),
                            Err(e) => format!("Error: {}", e),
                        }
                    }
                    "exists" => {
                        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                            match agent_tools.exists(path) {
                                Ok(exists) => format!("{{\"exists\": {}}}", exists),
                                Err(e) => format!("Error: {}", e),
                            }
                        } else {
                            "Error: Missing 'path' parameter".to_string()
                        }
                    }
                    _ => format!("Error: Unknown tool '{}'", function_name),
                }
            }
            Err(e) => format!("Error parsing arguments: {}", e),
        };

        results.push(format!("Tool '{}' result: {}", function_name, result));
    }

    results
}

pub async fn agent_mode(mistralrs: Arc<MistralRs>, do_search: bool) {
    let sender = mistralrs.get_sender(None).unwrap();
    let mut messages: Vec<IndexMap<String, MessageContent>> = Vec::new();

    let mut sampling_params = agent_sample_parameters();

    // Initialize agent tools with default sandbox configuration
    let agent_tools = AgentTools::with_defaults();
    info!(
        "Agent tools initialized with sandbox root: {}",
        agent_tools.config().root
    );

    info!("Starting agent mode with sampling params: {sampling_params:?}");
    println!("{}{}{}", "=".repeat(20), AGENT_MODE_HELP, "=".repeat(20));

    *CTRLC_HANDLER.lock().unwrap() = &exit_handler;

    ctrlc::set_handler(move || CTRLC_HANDLER.lock().unwrap()())
        .expect("Failed to set CTRL-C handler for agent mode");

    let mut rl = DefaultEditor::new().expect("Failed to open input");
    let _ = rl.load_history(&history_file_path());

    'outer: loop {
        *CTRLC_HANDLER.lock().unwrap() = &exit_handler;

        let prompt = read_line(&mut rl);
        let prompt_trimmed = prompt.as_str().trim();

        if prompt_trimmed.is_empty() {
            continue;
        }

        if handle_sampling_command(prompt_trimmed, &mut sampling_params) {
            continue;
        }

        // Handle commands
        match prompt_trimmed {
            HELP_CMD => {
                println!("{}{}{}", "=".repeat(20), AGENT_MODE_HELP, "=".repeat(20));
                continue;
            }
            EXIT_CMD => {
                break;
            }
            CLEAR_CMD => {
                messages.clear();
                info!("Cleared chat history.");
                continue;
            }
            _ => {
                // Add user message
                let mut user_message: IndexMap<String, MessageContent> = IndexMap::new();
                user_message.insert("role".to_string(), Either::Left("user".to_string()));
                user_message.insert(
                    "content".to_string(),
                    Either::Left(prompt_trimmed.to_string()),
                );
                messages.push(user_message);
            }
        }

        *CTRLC_HANDLER.lock().unwrap() = &terminate_handler;

        println!("\n{}", "=".repeat(60));
        println!("Processing query...");
        println!("{}", "=".repeat(60));

        let request_messages = RequestMessage::Chat {
            messages: messages.clone(),
            enable_thinking: None,
        };

        let (tx, mut rx) = channel(10_000);
        let req = Request::Normal(Box::new(NormalRequest {
            id: mistralrs.next_request_id(),
            messages: request_messages,
            sampling_params: sampling_params.clone(),
            response: tx,
            return_logprobs: false,
            is_streaming: true,
            constraint: Constraint::None,
            suffix: None,
            tool_choice: None,
            tools: None,
            logits_processors: None,
            return_raw_logits: false,
            web_search_options: do_search.then(WebSearchOptions::default),
            model_id: None,
        }));

        sender.send(req).await.unwrap();
        let start_ttft = Instant::now();
        let mut first_token_duration: Option<std::time::Duration> = None;

        let mut assistant_output = String::new();
        let mut last_usage = None;
        let mut tool_calls_detected = false;

        // Collect response chunks
        while let Some(resp) = rx.recv().await {
            match resp {
                Response::Chunk(chunk) => {
                    last_usage = chunk.usage.clone();
                    if let ChunkChoice {
                        delta:
                            Delta {
                                content: Some(content),
                                tool_calls: tool_calls_opt,
                                ..
                            },
                        finish_reason: finish_reason_opt,
                        ..
                    } = &chunk.choices[0]
                    {
                        if first_token_duration.is_none() {
                            let ttft = Instant::now().duration_since(start_ttft);
                            first_token_duration = Some(ttft);
                        }

                        assistant_output.push_str(content);
                        print!("{}", content);
                        io::stdout().flush().unwrap();

                        // Detect if tools were called
                        if tool_calls_opt.is_some() {
                            tool_calls_detected = true;
                        }

                        if let Some(reason) = finish_reason_opt {
                            if reason == "length" {
                                print!("...");
                            }
                            if reason == "tool_calls" {
                                println!("\n\n[Tools were executed automatically by the engine]");
                            }
                            break;
                        }
                    }
                }
                Response::InternalError(e) => {
                    error!("Got an internal error: {e:?}");
                    break 'outer;
                }
                Response::ModelError(e, resp) => {
                    error!("Got a model error: {e:?}, response: {resp:?}");
                    break 'outer;
                }
                Response::ValidationError(e) => {
                    error!("Got a validation error: {e:?}");
                    break 'outer;
                }
                Response::Done(_) => unreachable!(),
                Response::CompletionDone(_) => unreachable!(),
                Response::CompletionModelError(_, _) => unreachable!(),
                Response::CompletionChunk(_) => unreachable!(),
                Response::ImageGeneration(_) => unreachable!(),
                Response::Speech { .. } => unreachable!(),
                Response::Raw { .. } => unreachable!(),
            }
        }

        println!();

        // Print stats
        if let Some(last_usage) = last_usage {
            println!("\nStats:");
            if let Some(ttft) = first_token_duration {
                println!("  Time to first token: {:.2}s", ttft.as_secs_f32());
            }
            println!(
                "  Prompt: {} tokens, {:.2} T/s",
                last_usage.prompt_tokens, last_usage.avg_prompt_tok_per_sec
            );
            println!(
                "  Decode: {} tokens, {:.2} T/s",
                last_usage.completion_tokens, last_usage.avg_compl_tok_per_sec
            );
            if tool_calls_detected {
                println!("  Tool calls: Executed automatically");
            }
        }

        // Add assistant message to history
        let mut assistant_message: IndexMap<String, MessageContent> = IndexMap::new();
        assistant_message.insert("role".to_string(), Either::Left("assistant".to_string()));
        assistant_message.insert("content".to_string(), Either::Left(assistant_output));
        messages.push(assistant_message);

        println!("{}", "=".repeat(60));
        println!();
    }

    rl.save_history(&history_file_path()).unwrap();
}
