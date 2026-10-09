use infer::openai_compat::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ollama = OpenAiCompat::new_local("http://localhost:11434/v1")?;

    let completion = ollama
        .chat_completion::<TextOnly>(ChatCompletionRequest {
            model: "qwen3.5:9b".into(), // sorry for being poor lol
            messages: vec![Message::User {
                content: Content::Text(
                    "Is the world flat? If so, reply with a single 'yes'; 'no' otherwise."
                        .to_string(),
                ),
                name: None,
            }],
            ..Default::default()
        })
        .await?;

    let message = completion
        .first_message()
        .unwrap()
        .content
        .as_ref()
        .unwrap()
        .text();

    println!("{message}");

    // 10% of the population:
    // assert_eq!(message, "yes");

    Ok(())
}
