// Example demonstrating ReAct agent usage with mistral.rs
//
// This example shows how to:
// - Create a model with tool calling capabilities
// - Register custom tools with callbacks
// - Use the ReAct agent to autonomously execute multi-step reasoning
// - Track iteration history and final answers

use anyhow::Result;
use mistralrs::{Function, IsqType, ReActAgent, TextModelBuilder, Tool, ToolType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(serde::Deserialize, Debug, Clone)]
struct GetWeatherInput {
    place: String,
}

fn get_weather(input: GetWeatherInput) -> String {
    format!(
        "Weather in {}: Temperature: 25C. Wind: calm. Dew point: 10C. Precipitation: 5cm of rain expected.",
        input.place
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ReAct Agent Example ===\n");

    // Define the weather tool
    let weather_parameters: HashMap<String, Value> = serde_json::from_value(json!({
        "type": "object",
        "properties": {
            "place": {
                "type": "string",
                "description": "The place to get the weather for.",
            },
        },
        "required": ["place"],
    }))?;

    let weather_tool = Tool {
        tp: ToolType::Function,
        function: Function {
            description: Some("Get the weather for a certain city.".to_string()),
            name: "get_weather".to_string(),
            parameters: Some(weather_parameters),
        },
    };

    // Create weather tool callback
    let weather_callback = Arc::new(|called_function: &mistralrs_core::CalledFunction| {
        let input: GetWeatherInput = serde_json::from_str(&called_function.arguments)?;
        Ok(get_weather(input))
    });

    // Build the model with tool callback
    println!("Building model with tool callback...");
    let model = TextModelBuilder::new("meta-llama/Meta-Llama-3.1-8B-Instruct")
        .with_logging()
        .with_isq(IsqType::Q8_0)
        .with_tool_callback_and_tool("get_weather", weather_callback, weather_tool)
        .build()
        .await?;

    println!("Model built successfully!\n");

    // Create ReAct agent with configuration
    // ReActAgent::new returns Result, so unwrap (propagate anyhow via ? earlier) then chain config
    let agent = ReActAgent::new(model)?
        .with_max_iterations(5)
        .with_tool_timeout_secs(30);

    // Run the agent
    println!("Running ReAct agent with query: 'What is the weather in Boston and Paris?'\n");
    let response = agent
        .run("What is the weather in Boston and Paris? Compare them.")
        .await?;

    // Display results
    println!("\n=== Agent Results ===\n");
    println!("Final Answer: {}\n", response.final_answer);
    println!("Total Iterations: {}\n", response.total_iterations);

    // Show iteration details
    for (i, iteration) in response.iterations.iter().enumerate() {
        println!("--- Iteration {} ---", i + 1);

        if let Some(ref thought) = iteration.thought {
            println!("Thought: {}", thought);
        }

        println!("Actions: {} tool call(s)", iteration.actions.len());
        for (j, action) in iteration.actions.iter().enumerate() {
            println!(
                "  {}. Tool: {} | Arguments: {}",
                j + 1,
                action.function.name,
                action.function.arguments
            );
        }

        println!("Observations:");
        for (j, obs) in iteration.observations.iter().enumerate() {
            println!("  {}. Result: {}", j + 1, obs.content);
            if let Some(ref error) = obs.error {
                println!("     Error: {}", error);
            }
        }
        println!();
    }

    Ok(())
}
