use futures_util::StreamExt;
use infer::openai_compat::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ollama = OpenAiCompat::new_local("http://localhost:11434/v1")?;

    let mut completion = ollama
        .chat_completion::<StreamingCompletion>(&ChatCompletionRequest {
            model: "qwen3.5:9b".into(), // sorry for being poor lol
            messages: vec![
                Message::System {
                    content: Content::Text(
                        "You're tab, an AI agent capable of helping the user with research tasks."
                            .to_string(),
                    ),
                    name: None,
                },
                Message::User {
                    content: Content::Text("Introduce yourself a bit".to_string()),
                    name: None,
                },
            ],
            ..Default::default()
        })
        .await?;

    while let Some(chunk) = completion.next().await {
        for choice in chunk?.choices {
            if let Some(text) = choice.delta.content {
                print!("{text}");
            }
        }
    }
    println!();

    Ok(())
}
