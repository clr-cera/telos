use std::{env, process::exit, sync::Arc};

use dotenv::dotenv;
use teloxide::prelude::*;

mod db;
mod handler;
mod admin;
mod miguel;

async fn create_db() -> Result<db::DB, Box<dyn std::error::Error>> {
    let path = env::var("DATABASE_PATH").unwrap_or_else(|_| "sqlite://db.sqlite?mode=rwc".to_string());

    let db = db::DB::new(&path).await?;
    db.migrate().await?;

    log::info!("Database created at {:?}", path);

    Ok(db)
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    pretty_env_logger::init();

    log::info!("Starting bot...");

    let token = match env::var("TELOXIDE_TOKEN") {
        Ok(token) => token,
        Err(e) => {
            log::error!("Error getting TELOXIDE_TOKEN: {:?}", e);
            exit(1);
        }
    };

    let bot = Bot::new(token);

    let db = match create_db().await {
        Ok(db) => db,
        Err(e) => {
            log::error!("Error creating database: {:?}", e);
            exit(1);
        }
    };


    let miguel_handler = match miguel::MiguelHandler::new() {
        Some(handler) => handler,
        None => {
            log::error!("Error creating miguel handler");
            exit(1);
        }
    };

    let admin_handler = match admin::AdminHandler::new(db.clone()) {
        Some(handler) => handler,
        None => {
            log::error!("Error creating admin handler");
            exit(1);
        }
    };

    let the_handler = Arc::new(handler::Handler::new(miguel_handler, admin_handler));

    let schema = {
        let handler_clone = Arc::clone(&the_handler); // move clone into closure

        Update::filter_message().branch(
            Message::filter_text().endpoint(
                move |bot: Bot, msg: Message| {
                    let handler_clone = Arc::clone(&handler_clone); // clone inside closure

                    async move {
                        handler_clone.handle(&bot, &msg).await?;
                        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                    }
                },
            ),
        )
    };

    Dispatcher::builder(bot, schema).build().dispatch().await;
}
