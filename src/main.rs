use dotenv::dotenv;
use teloxide::{prelude::*, types::ReactionType};
use regex::Regex;

const MIGUEL: &str = "
PAROU, PAROU A DISCUSSÃO

miguwu ><
";

const MIGUEL_USER: &str = "migeyel";


#[tokio::main]
async fn main() {
    dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();
    let schema = Update::filter_message().branch(Message::filter_text().endpoint(answer));

    Dispatcher::builder(bot, schema).build().dispatch().await;
}

async fn answer(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = msg.text().unwrap_or_default().to_lowercase();


    handle_miguel_command(&bot, &msg, &text).await?;
    handle_miguel_message(&bot, &msg, &text).await?;
    

    Ok(())
}

async fn handle_miguel_command(bot: &Bot, msg: &Message, text: &str) -> ResponseResult<()> {
    let miguel_re: Regex = match Regex::new(r"/[A-z0-9À-ÿ]*?miguel[A-z0-9À-ÿ]*") {
        Ok(re) => re,
        Err(e) => {
            log::error!("Error creating regex: {:?}", e);
            return Ok(());
        }
    };

    if !miguel_re.is_match(text) {
        return Ok(());
    }

    log::info!("Received **miguel** command: {:?}", msg);

    match msg.thread_id {
        Some(thread_id) => {
            bot.send_message(msg.chat.id, MIGUEL)
                .message_thread_id(thread_id)
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, MIGUEL).await?;
        }
    }

    Ok(())
}

async fn handle_miguel_message(bot: &Bot, msg: &Message, text: &str) -> ResponseResult<()> {
    // Extract message sender
    if msg.from.is_none() {
        return Ok(());
    }

    let from = msg.from.as_ref().unwrap().clone();

    // Check if message is from miguel
    if 
        from.username.is_none()
        || from.username.unwrap() != MIGUEL_USER
    {
        return Ok(());
    }



    // Check if this is a "miguel" message
    if !text.contains("miguel") && !text.contains("miguwu") {
        return Ok(());
    }


    // Received a miguel, react with 🗿
    log::info!("A wild **miguel** appeared: {:?}", msg);

    bot.set_message_reaction(msg.chat.id, msg.id)
        .reaction([ReactionType::Emoji { emoji: "🗿".to_string() }])
        .await?;

    Ok(())
}
