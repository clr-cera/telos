use teloxide::{prelude::*, types::ReactionType};

use regex::Regex;

const MIGUEL: &str = "
PAROU, PAROU A DISCUSSÃO

miguwu ><
";

const MIGUEL_USER: &str = "migeyel";

#[derive(Clone)]
pub struct MiguelHandler {
    miguel_re: Regex
}

impl MiguelHandler {
    pub fn new() -> Option<Self> {
        let miguel_re: Regex = match Regex::new(r"/[A-z0-9À-ÿ]*?miguel[A-z0-9À-ÿ]*") {
            Ok(re) => re,
            Err(e) => {
                log::error!("Error creating regex: {:?}", e);
                return None;
            }
        };

        Some(Self { miguel_re })
    }

    pub async fn handle(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        let text = msg.text().unwrap_or_default().to_lowercase();

        self.handle_miguel_command(bot, msg, &text).await?;
        self.handle_miguel_message(bot, msg, &text).await?;

        Ok(())
    }


    pub async fn handle_miguel_command(&self, bot: &Bot, msg: &Message, text: &str) -> ResponseResult<()> {
        if !self.miguel_re.is_match(text) {
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

    pub async fn handle_miguel_message(&self, bot: &Bot, msg: &Message, text: &str) -> ResponseResult<()> {
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
}


