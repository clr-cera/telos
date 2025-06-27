use dotenv::dotenv;
use teloxide::prelude::*;

const MIGUEL: &str = "
PAROU, PAROU A DISCUSSÃO

miguwu ><
";

#[tokio::main]
async fn main() {
    println!("Starting bot...");
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    let schema = Update::filter_message().branch(Message::filter_text().endpoint(answer));

    Dispatcher::builder(bot, schema).build().dispatch().await;
}

async fn answer(bot: Bot, msg: Message) -> ResponseResult<()> {
    println!("Received message: {:?}", msg);
    let text = msg.text().unwrap_or_default().to_lowercase();
    if text.contains("miguel") {
        bot.send_message(msg.chat.id, MIGUEL).await?;
        return Ok(());
    }
    Ok(())
}
