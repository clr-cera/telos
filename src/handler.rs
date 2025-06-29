use teloxide::{prelude::*};
use crate::{admin::AdminHandler, miguel::MiguelHandler};


pub struct Handler {
    miguel_handler: MiguelHandler,
    admin_handler: AdminHandler,
}

impl Handler {
    pub fn new(miguel_handler: MiguelHandler, admin_handler: AdminHandler) -> Self {
        Self { miguel_handler, admin_handler }
    }



    pub async fn handle(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        // Admin commands are immune to whitelisting
        self.admin_handler.handle(bot, msg).await?;

        // Check whitelisting
        match self.admin_handler.check_whitelist(msg).await {
            Ok(true) => {}
            Ok(false) => {
                log::trace!("Message is not whitelisted: {:?}", msg);
                return Ok(());
            }
            Err(e) => {
                log::error!("Error checking if message is whitelisted: {:?}", e);
                return Ok(());
            }
        }

        self.miguel_handler.handle(bot, msg).await?;

        Ok(())
    }
}

