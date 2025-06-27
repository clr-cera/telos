use dotenv::dotenv;
use teloxide::{prelude::*, utils::command::BotCommands};

const MIGUEL: &str = "
PAROU, PAROU A DISCUSSÃO

miguwu ><
";

#[tokio::main]
async fn main() {
    dotenv().ok();
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Say hello")]
    Hello,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let text = msg.text().unwrap_or_default().to_lowercase();
    if text.contains("miguel") {
        bot.send_message(msg.chat.id, MIGUEL).await?;
        return Ok(());
    }
    match cmd {
        Command::Hello => {
            bot.send_message(msg.chat.id, "Hello!").await?;
        }
    }
    Ok(())
}
