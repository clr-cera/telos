use teloxide::{prelude::*, types::User};

use crate::db;


pub struct AdminHandler {
    db: db::DB,
}

impl AdminHandler {
    pub fn new(db: db::DB) -> Option<Self> {
        Some(Self { db })
    }

    pub async fn check_whitelist(&self, msg: &Message) -> Result<bool, Box<dyn std::error::Error>> {
        if msg.chat.is_group() {
            match self.is_group_allowed(msg.chat.id.0).await {
                Ok(true) => {}
                Ok(false) => {
                    log::info!("Group is not whitelisted: {:?}", msg);
                    return Ok(false);
                }
                Err(e) => {
                    log::error!("Error checking if group is whitelisted: {:?}", e);
                    return Err(e.into());
                }
            }

            if let Some(thread_id) = msg.thread_id {
                match self.is_thread_allowed(thread_id.0.0, msg.chat.id.0).await {
                    Ok(true) => {}
                    Ok(false) => {
                        log::info!("Thread is not whitelisted: {:?}", msg);
                        return Ok(false);
                    }
                    Err(e) => {
                        log::error!("Error checking if thread is whitelisted: {:?}", e);
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(true)
    }

    pub async fn is_group_allowed(&self, group_id: i64) -> Result<bool, rusqlite::Error> {
        self.db.is_group_whitelisted(group_id).await
    }

    pub async fn is_thread_allowed(&self, thread_id: i32, group_id: i64) -> Result<bool, rusqlite::Error> {
        self.db.is_thread_whitelisted(thread_id, group_id).await
    }

    pub async fn handle(&self, bot: &Bot, msg: &Message) -> Result<(), teloxide::RequestError> {
        if msg.from.is_none() {
            return Ok(());
        }

        let from = msg.from.as_ref().unwrap().clone();

        let admin = match self.db.get_admin(from.id.0).await {
            Ok(Some(admin)) => admin,
            Ok(None) => {
                log::info!("User is not admin: {:?}", msg);
                return Ok(());
            }
            Err(e) => {
                log::error!("Error checking if user is admin: {:?}", e);
                return Ok(());
            }
        };

        let text = msg.text().unwrap_or_default().to_lowercase();

        let cmd = match text.split_once(" ") {
            None => text.as_str(),
            Some((cmd, _)) => cmd,
        };

        match cmd {
            "/add_admin" => self.add_admin(bot, msg, from.id.0).await?,
            "/whitelist_group" => self.whitelist_group(bot, msg, from.id.0).await?,
            "/whitelist_thread" => self.whitelist_thread(bot, msg, from.id.0).await?,
            "/unwhitelist_group" => self.unwhitelist_group(bot, msg, from.id.0).await?,
            "/unwhitelist_thread" => self.unwhitelist_thread(bot, msg, from.id.0).await?,
            "/remove_admin" => self.remove_admin(bot, msg, admin).await?,
            "/make_superadmin" => self.make_superadmin(bot, msg, admin).await?,
            "/list_admins" => self.list_admins(bot, msg).await?,
            "/list_whitelisted_groups" => self.list_whitelisted_groups(bot, msg).await?,
            "/list_whitelisted_threads" => self.list_whitelisted_threads(bot, msg).await?,
            "/help" => self.help(bot, msg).await?,
            &_ => {
                return Ok(());
            }
        }


        Ok(())
    }

    async fn add_admin(&self, bot: &Bot, msg: &Message, admin_id: u64) -> Result<(), teloxide::RequestError> {
        // This command is only valid in private chats
        if !msg.chat.is_private() {
            return Ok(());
        }

        let mentions: Vec<&User> = msg.mentioned_users().take(2).collect();

        if mentions.len() != 1 {
            bot.send_message(msg.chat.id, "Invalid command, use /add_admin <@username>").await?;
            return Ok(());
        }

        let user_id = mentions[0].id.0;
        let user_name= mentions[0].username.as_deref();

        match self.db.add_admin(
                user_id, 
            admin_id,
                user_name,
            ).await {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Admin added!").await?;
            }
            Err(e) => {
                log::error!("Error adding admin: {:?}", e);
                bot.send_message(msg.chat.id, "Error adding admin!").await?;
            }
        }

        Ok(())
    }


    async fn whitelist_group(&self, bot: &Bot, msg: &Message, admin_id: u64) -> Result<(), teloxide::RequestError> {
        // This command is only valid in groups
        if !msg.chat.is_group() {
            return Ok(());
        }

        let group_id = msg.chat.id.0;
        let group_name= msg.chat.title();

        match self.db.add_whitelisted_group(
            group_id, 
            admin_id,
            group_name,
        ).await {
            Ok(_) => {
                let mut reply = bot.send_message(msg.chat.id, "Group whitelisted!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error whitelisting group: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error whitelisting group!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn whitelist_thread(&self, bot: &Bot, msg: &Message, admin_id: u64) -> Result<(), teloxide::RequestError> {
        // This command is only valid in private chats
        if !msg.chat.is_group() {
            return Ok(());
        }

        let group_id = msg.chat.id.0;
        let thread_id = match msg.thread_id {
            Some(thread_id) => thread_id,
            None => {
                bot.send_message(msg.chat.id, "Can only be used in threads!").await?;
                return Ok(());
            }
        };

        let group_name= msg.chat.title();
        let thread_name = None;

        match self.db.add_whitelisted_thread(
            thread_id.0.0, 
            group_id, 
            admin_id,
            group_name,
            thread_name,
        ).await {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Thread whitelisted!")
                    .message_thread_id(thread_id)
                    .await?;
            }
            Err(e) => {
                log::error!("Error whitelisting thread: {:?}", e);
                bot.send_message(msg.chat.id, "Error whitelisting thread!")
                    .message_thread_id(thread_id)
                    .await?;
            }
        }

        Ok(())
    }

    async fn unwhitelist_group(&self, bot: &Bot, msg: &Message, _admin_id: u64) -> ResponseResult<()> {
        let text = msg.text().unwrap_or_default().to_lowercase();

        let group_id = if msg.chat.is_group() {
            msg.chat.id.0
        } else {
            // Parse form the text
            match text.split_whitespace().nth(1) {
                Some(group_id) => {
                    match group_id.parse::<i64>() {
                        Ok(group_id) => group_id,
                        Err(_) => {
                            bot.send_message(msg.chat.id, "Invalid group id").await?;
                            return Ok(());
                        }
                    }
                }
                None => {
                    bot.send_message(msg.chat.id, "Invalid command, use /unwhitelist_group <@group_id>").await?;
                    return Ok(());
                }
            }
        };

        match self.db.remove_whitelisted_group(group_id).await {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Group unwhitelisted!").await?;
            }
            Err(e) => {
                log::error!("Error unwhitelisting group: {:?}", e);
                bot.send_message(msg.chat.id, "Error unwhitelisting group!").await?;
            }
        }

        Ok(())
    }

    async fn unwhitelist_thread(&self, bot: &Bot, msg: &Message, _admin_id: u64) -> ResponseResult<()> {
        let text = msg.text().unwrap_or_default().to_lowercase();
        let args =  text.split_whitespace().collect::<Vec<&str>>();

        let (group_id, thread_id) = match args.len() {
            1 => {
                if !msg.chat.is_group() {
                    bot.send_message(msg.chat.id, "Invalid command: Use /unwhitelist_thread <@group_id> <@thread_id>").await?;
                    return Ok(());
                }

                match msg.thread_id {
                    Some(thread_id_val) => (msg.chat.id.0, thread_id_val.0.0),
                    None => {
                        bot.send_message(
                            msg.chat.id,
                            "Invalid command: Use /unwhitelist_thread <@thread_id> in threads or provide group ID"
                        ).await?;
                        return Ok(());
                    }
                }
            },
            2 => {
                if !msg.chat.is_group() {
                    bot.send_message(msg.chat.id, "Invalid command: Use /unwhitelist_thread <@group_id> <@thread_id>").await?;
                    return Ok(());
                }

                let thread_id = match args[1].parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid thread ID format").await?;
                        return Ok(());
                    }
                };

                (msg.chat.id.0, thread_id)
            },
            3 => {
                let group_id = match args[1].parse::<i64>() {
                    Ok(id) => id,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid group ID format").await?;
                        return Ok(());
                    }
                };
                let thread_id = match args[2].parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid thread ID format").await?;
                        return Ok(());
                    }
                };
                (group_id, thread_id)
            },
            _ => {
                bot.send_message(
                    msg.chat.id,
                    "Invalid command format. Usage:\n/unwhitelist_thread\n/unwhitelist_thread <thread_id>\n/unwhitelist_thread <group_id> <thread_id>"
                ).await?;
                return Ok(());
            }
        };

        match self.db.remove_whitelisted_thread(thread_id, group_id).await {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Thread unwhitelisted!").await?;
            }
            Err(e) => {
                log::error!("Error unwhitelisting thread: {:?}", e);
                bot.send_message(msg.chat.id, "Error unwhitelisting thread!").await?;
            }
        }

        Ok(())
    }

    async fn remove_admin(&self, bot: &Bot, msg: &Message, admin: db::Admin) -> ResponseResult<()> {
        let text = msg.text().unwrap_or_default().to_lowercase();

        let user_id = match text.split_whitespace().nth(1) {
            Some(user_id) => {
                match user_id.parse::<u64>() {
                    Ok(user_id) => user_id,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid user id").await?;
                        return Ok(());
                    }
                }
            },
            None => {
                bot.send_message(msg.chat.id, "Invalid command, use /remove_admin <@user_id>").await?;
                return Ok(());
            }
        };

        
        if admin.is_superadmin() {
            match self.db.remove_admin(user_id).await {
                Ok(_) => {
                    bot.send_message(msg.chat.id, "Admin removed!").await?;
                }
                Err(e) => {
                    log::error!("Error removing admin: {:?}", e);
                    bot.send_message(msg.chat.id, "Error removing admin!").await?;
                }
            }
        } else {
            match self.db.remove_admin_with_traversal(user_id, admin.user_id).await {
                Ok(true) => {
                    bot.send_message(msg.chat.id, "Admin removed!").await?;
                }
                Ok(false) => {
                    bot.send_message(msg.chat.id, "You are not an admin of this user").await?;
                }
                Err(e) => {
                    log::error!("Error removing admin: {:?}", e);
                    bot.send_message(msg.chat.id, "Error removing admin!").await?;
                }
            }
        }

        Ok(())
    }


    async fn make_superadmin(&self, bot: &Bot, msg: &Message, admin: db::Admin) -> ResponseResult<()> {
        // This command is only valid in private chats
        if !msg.chat.is_private() {
            return Ok(());
        }

        if !admin.is_superadmin() {
            bot.send_message(msg.chat.id, "You are not a superadmin").await?;
            return Ok(());
        }


        let text = msg.text().unwrap_or_default().to_lowercase();

        let target_id = text.split_whitespace().nth(1);

        match target_id {
            Some(target_id) => {
                match target_id.parse::<u64>() {
                    Ok(target_id) => {
                        match self.db.make_superadmin(target_id).await {
                            Ok(_) => {
                                bot.send_message(msg.chat.id, "Superadmin made!").await?;
                            }
                            Err(e) => {
                                log::error!("Error making superadmin: {:?}", e);
                                bot.send_message(msg.chat.id, "Error making superadmin!").await?;
                            }
                        }
                    },
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid user id").await?;
                        return Ok(());
                    }
                }
            },
            None => {
                bot.send_message(msg.chat.id, "Invalid command, use /make_superadmin <@user_id>").await?;
                return Ok(());
            }
        }

        Ok(())
    }

    async fn list_admins(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        let admins = match self.db.get_admins().await {
            Ok(admins) => admins,
            Err(e) => {
                log::error!("Error listing admins: {:?}", e);
                bot.send_message(msg.chat.id, "Error listing admins!").await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Admins:".to_string()];
        for admin in admins {
            message_lines.push(format!("{:?}", admin));
        }
        
        bot.send_message(msg.chat.id, message_lines.join("\n")).await?; 

        Ok(())
    }

    async fn list_whitelisted_groups(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        let groups = match self.db.get_whitelisted_groups().await {
            Ok(groups) => groups,
            Err(e) => {
                log::error!("Error listing whitelisted groups: {:?}", e);
                bot.send_message(msg.chat.id, "Error listing whitelisted groups!").await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Whitelisted groups:".to_string()];
        for group in groups {
            message_lines.push(format!("{:?}", group));
        }

        bot.send_message(msg.chat.id, message_lines.join("\n")).await?;

        Ok(())
    }

    async fn list_whitelisted_threads(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        let threads = match self.db.get_whitelisted_threads(msg.chat.id.0).await {
            Ok(threads) => threads,
            Err(e) => {
                log::error!("Error listing whitelisted threads: {:?}", e);
                bot.send_message(msg.chat.id, "Error listing whitelisted threads!").await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Whitelisted threads:".to_string()];
        for thread in threads {
            message_lines.push(format!("{:?}", thread));
        }

        bot.send_message(msg.chat.id, message_lines.join("\n")).await?;

        Ok(())
    }

    async fn help(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {

        let help = r#"
/add_admin <@user_id> [<@user_name>]
/whitelist_group <@group_id> [<@group_name>]
/whitelist_thread <@group_id> <@thread_id> [<@thread_name>]
/unwhitelist_group <@group_id>
/unwhitelist_thread <@group_id> <@thread_id>
/remove_admin <@user_id>
/make_superadmin <@user_id>
/list_admins
/list_whitelisted_groups
/list_whitelisted_threads
"#;

        bot.send_message(msg.chat.id, help).await?;

        Ok(())
    }
}
