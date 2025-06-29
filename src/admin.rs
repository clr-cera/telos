use teloxide::{prelude::*};

use crate::db;


pub struct AdminHandler {
    db: db::DB,
}

impl AdminHandler {
    pub fn new(db: db::DB) -> Option<Self> {
        Some(Self { db })
    }

    pub async fn check_whitelist(&self, msg: &Message) -> Result<bool, Box<dyn std::error::Error>> {
        if msg.chat.is_group() || msg.chat.is_supergroup() {
            match self.is_group_allowed(msg.chat.id.0).await {
                Ok(true) => {}
                Ok(false) => {
                    log::trace!("Group is not whitelisted: {:?}", msg);
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
                        log::trace!("Thread is not whitelisted: {:?}", msg);
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

    pub async fn is_group_allowed(&self, group_id: i64) -> Result<bool, sqlx::Error> {
        self.db.is_group_whitelisted(group_id).await
    }

    pub async fn is_thread_allowed(&self, thread_id: i32, group_id: i64) -> Result<bool, sqlx::Error> {
        self.db.is_thread_whitelisted(thread_id, group_id).await
    }

    pub async fn handle(&self, bot: &Bot, msg: &Message) -> Result<(), teloxide::RequestError> {
        if msg.from.is_none() {
            return Ok(());
        }

        let from = msg.from.as_ref().unwrap().clone();

        let text = msg.text().unwrap_or_default().to_lowercase();

        let cmd = match text.split_once(" ") {
            None => text.as_str(),
            Some((cmd, _)) => cmd,
        };

        let admin = match self.db.get_admin(from.id.0 as i64).await {
            Ok(Some(admin)) => admin,
            Ok(None) => {
                if cmd == "/become_admin" {
                    return self.become_admin(bot, msg).await;
                }

                log::info!("User is not admin: {:?}", msg);
                return Ok(());
            }
            Err(e) => {
                log::error!("Error checking if user is admin: {:?}", e);
                return Ok(());
            }
        };


        log::trace!("Command: {cmd:?}");
        log::trace!("Text: {text:?}");

        match cmd {
            "/whitelist_group" => self.whitelist_group(bot, msg, from.id.0).await?,
            "/whitelist_thread" => self.whitelist_thread(bot, msg, from.id.0).await?,
            "/unwhitelist_group" => self.unwhitelist_group(bot, msg, from.id.0).await?,
            "/unwhitelist_thread" => self.unwhitelist_thread(bot, msg, from.id.0).await?,
            "/remove_admin" => self.remove_admin(bot, msg, admin).await?,
            "/make_superadmin" => self.make_superadmin(bot, msg, admin).await?,
            "/list_admins" => self.list_admins(bot, msg).await?,
            "/list_whitelisted_groups" => self.list_whitelisted_groups(bot, msg).await?,
            "/list_whitelisted_threads" => self.list_whitelisted_threads(bot, msg).await?,
            "/approve_become_admin" => self.approve_become_admin(bot, msg, from.id.0).await?,
            "/reject_become_admin" => self.reject_become_admin(bot, msg, from.id.0).await?,
            "/list_become_admin_requests" => self.list_become_admin_requests(bot, msg).await?,
            "/help" => self.help(bot, msg).await?,
            &_ => {
                return Ok(());
            }
        }


        Ok(())
    }

    async fn whitelist_group(&self, bot: &Bot, msg: &Message, admin_id: u64) -> Result<(), teloxide::RequestError> {
        log::trace!("Whtelisting group: {:?}", msg);
        // This command is only valid in groups
        if !msg.chat.is_group() && !msg.chat.is_supergroup() {
            return Ok(());
        }

        let group_id = msg.chat.id.0;
        let group_name= msg.chat.title();

        match self.db.add_whitelisted_group(
            group_id, 
            admin_id as i64,
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
        log::trace!("Whtelisting thread: {:?}", msg);
        if !msg.chat.is_supergroup() {
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
            admin_id as i64,
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
        log::trace!("Unwhitelisting group: {:?}", msg);
        let text = msg.text().unwrap_or_default().to_lowercase();

        let group_id = if msg.chat.is_group() || msg.chat.is_supergroup() {
            msg.chat.id.0
        } else {
            // Parse form the text
            match text.split_whitespace().nth(1) {
                Some(group_id) => {
                    match group_id.parse::<i64>() {
                        Ok(group_id) => group_id,
                        Err(_) => {
                            let mut reply = bot.send_message(msg.chat.id, "Invalid group id");
                            if let Some(thread_id) = msg.thread_id {
                                reply = reply.message_thread_id(thread_id);
                            }
                            reply.await?;
                            return Ok(());
                        }
                    }
                }
                None => {
                    let mut reply = bot.send_message(msg.chat.id, "Invalid command, use /unwhitelist_group <@group_id>");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                    return Ok(());
                }
            }
        };

        match self.db.remove_whitelisted_group(group_id).await {
            Ok(_) => {
                let mut reply = bot.send_message(msg.chat.id, "Group unwhitelisted!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error unwhitelisting group: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error unwhitelisting group!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn unwhitelist_thread(&self, bot: &Bot, msg: &Message, _admin_id: u64) -> ResponseResult<()> {
        log::trace!("Unwhitelisting thread: {:?}", msg);
        let text = msg.text().unwrap_or_default().to_lowercase();
        let args =  text.split_whitespace().collect::<Vec<&str>>();

        let (group_id, thread_id) = match args.len() {
            1 => {
                if !msg.chat.is_group() && !msg.chat.is_supergroup() {
                    let mut reply = bot.send_message(msg.chat.id, "Invalid command: Use /unwhitelist_thread <@group_id> <@thread_id>");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
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
                if !msg.chat.is_group() && !msg.chat.is_supergroup() {
                    bot.send_message(msg.chat.id, "Invalid command: Use /unwhitelist_thread <@group_id> <@thread_id>").await?;
                    return Ok(());
                }

                let thread_id = match args[1].parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid thread ID format");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                };

                (msg.chat.id.0, thread_id)
            },
            3 => {
                let group_id = match args[1].parse::<i64>() {
                    Ok(id) => id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid group ID format");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                };
                let thread_id = match args[2].parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid thread ID format");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                };
                (group_id, thread_id)
            },
            _ => {
                let mut reply = bot.send_message(
                    msg.chat.id,
                    "Invalid command format. Usage:\n/unwhitelist_thread\n/unwhitelist_thread <thread_id>\n/unwhitelist_thread <group_id> <thread_id>"
                );
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        match self.db.remove_whitelisted_thread(thread_id, group_id).await {
            Ok(_) => {
                let mut reply = bot.send_message(msg.chat.id, "Thread unwhitelisted!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error unwhitelisting thread: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error unwhitelisting thread!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn remove_admin(&self, bot: &Bot, msg: &Message, admin: db::Admin) -> ResponseResult<()> {
        log::trace!("Removing admin: {:?}", msg);
        let text = msg.text().unwrap_or_default().to_lowercase();

        let user_id = match text.split_whitespace().nth(1) {
            Some(user_id) => {
                match user_id.parse::<u64>() {
                    Ok(user_id) => user_id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid user id");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                }
            },
            None => {
                let mut reply = bot.send_message(msg.chat.id, "Invalid command, use /remove_admin <@user_id>");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        
        if admin.is_superadmin() {
            match self.db.remove_admin(user_id as i64).await {
                Ok(_) => {
                    let mut reply = bot.send_message(msg.chat.id, "Admin removed!");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                }
                Err(e) => {
                    log::error!("Error removing admin: {:?}", e);
                    let mut reply = bot.send_message(msg.chat.id, "Error removing admin!");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                }
            }
        } else {
            match self.db.remove_admin_with_traversal(user_id as i64, admin.user_id).await {
                Ok(true) => {
                    let mut reply = bot.send_message(msg.chat.id, "Admin removed!");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                }
                Ok(false) => {
                    let mut reply = bot.send_message(msg.chat.id, "You are not an admin of this user");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                }
                Err(e) => {
                    log::error!("Error removing admin: {:?}", e);
                    let mut reply = bot.send_message(msg.chat.id, "Error removing admin!");
                    if let Some(thread_id) = msg.thread_id {
                        reply = reply.message_thread_id(thread_id);
                    }
                    reply.await?;
                }
            }
        }

        Ok(())
    }


    async fn make_superadmin(&self, bot: &Bot, msg: &Message, admin: db::Admin) -> ResponseResult<()> {
        log::trace!("Making superadmin: {:?}", msg);
        // This command is only valid in private chats
        if !admin.is_superadmin() {
            let mut reply = bot.send_message(msg.chat.id, "You are not a superadmin");
            if let Some(thread_id) = msg.thread_id {
                reply = reply.message_thread_id(thread_id);
            }
            reply.await?;
            return Ok(());
        }


        let text = msg.text().unwrap_or_default().to_lowercase();

        let target_id = text.split_whitespace().nth(1);

        match target_id {
            Some(target_id) => {
                match target_id.parse::<u64>() {
                    Ok(target_id) => {
                        match self.db.make_superadmin(target_id as i64).await {
                            Ok(_) => {
                                let mut reply = bot.send_message(msg.chat.id, "Superadmin made!");
                                if let Some(thread_id) = msg.thread_id {
                                    reply = reply.message_thread_id(thread_id);
                                }
                                reply.await?;
                            }
                            Err(e) => {
                                log::error!("Error making superadmin: {:?}", e);
                                let mut reply = bot.send_message(msg.chat.id, "Error making superadmin!");
                                if let Some(thread_id) = msg.thread_id {
                                    reply = reply.message_thread_id(thread_id);
                                }
                                reply.await?;
                            }
                        }
                    },
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid user id");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                }
            },
            None => {
                let mut reply = bot.send_message(msg.chat.id, "Invalid command, use /make_superadmin <@user_id>");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        }

        Ok(())
    }

    async fn list_admins(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        log::trace!("Listing admins: {:?}", msg);

        let admins = match self.db.get_admins().await {
            Ok(admins) => admins,
            Err(e) => {
                log::error!("Error listing admins: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error listing admins!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Admins:".to_string()];
        for admin in admins {
            message_lines.push(format!("{:?}", admin));
        }
        
        let mut reply = bot.send_message(msg.chat.id, message_lines.join("\n"));
        if let Some(thread_id) = msg.thread_id {
            reply = reply.message_thread_id(thread_id);
        }
        reply.await?;

        Ok(())
    }

    async fn list_whitelisted_groups(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        log::trace!("Listing whitelisted groups: {:?}", msg);
        let groups = match self.db.get_whitelisted_groups().await {
            Ok(groups) => groups,
            Err(e) => {
                log::error!("Error listing whitelisted groups: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error listing whitelisted groups!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Whitelisted groups:".to_string()];
        for group in groups {
            message_lines.push(format!("{:?}", group));
        }

        let mut reply = bot.send_message(msg.chat.id, message_lines.join("\n"));
        if let Some(thread_id) = msg.thread_id {
            reply = reply.message_thread_id(thread_id);
        }
        reply.await?;

        Ok(())
    }

    async fn list_whitelisted_threads(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        log::trace!("Listing whitelisted threads: {:?}", msg);
        let threads = match self.db.get_whitelisted_threads(msg.chat.id.0).await {
            Ok(threads) => threads,
            Err(e) => {
                log::error!("Error listing whitelisted threads: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error listing whitelisted threads!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Whitelisted threads:".to_string()];
        for thread in threads {
            message_lines.push(format!("{:?}", thread));
        }

        let mut reply = bot.send_message(msg.chat.id, message_lines.join("\n"));
        if let Some(thread_id) = msg.thread_id {
            reply = reply.message_thread_id(thread_id);
        }
        reply.await?;

        Ok(())
    }

    async fn become_admin(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        log::trace!("Becoming admin: {:?}", msg);

        if msg.from.is_none() {
            return Ok(());
        }

        let user_id = msg.from.as_ref().unwrap().id.0;
        let user_name= msg.from.as_ref().unwrap().username.as_deref();


        match self.db.create_become_admin_request(user_id as i64, user_name).await {
            Ok(Some(request_id)) => {
                let mut reply = bot.send_message(msg.chat.id, format!("Request created, use /approve_become_admin <{}> to approve", request_id));
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Ok(None) => {
                let mut reply = bot.send_message(msg.chat.id, "Error creating request");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error creating become admin request: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error creating become admin request!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn approve_become_admin(&self, bot: &Bot, msg: &Message, admin_id: u64) -> ResponseResult<()> {
        log::trace!("Approving become admin: {:?}", msg);
        let text = msg.text().unwrap_or_default().to_lowercase();

        let request_id = match text.split_whitespace().nth(1) {
            Some(request_id) => {
                match request_id.parse::<String>() {
                    Ok(request_id) => request_id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid request id");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                }
            },
            None => {
                let mut reply = bot.send_message(msg.chat.id, "Invalid command, use /approve_become_admin <request_id>");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        match self.db.approve_become_admin_request(&request_id, admin_id as i64).await {
            Ok(_) => {
                let mut reply = bot.send_message(msg.chat.id, "Admin approved!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error approving become admin request: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error approving become admin request!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn reject_become_admin(&self, bot: &Bot, msg: &Message, _admin_id: u64) -> ResponseResult<()> {
        log::trace!("Rejecting become admin: {:?}", msg);
        let text = msg.text().unwrap_or_default().to_lowercase();

        let request_id = match text.split_whitespace().nth(1) {
            Some(request_id) => {
                match request_id.parse::<String>() {
                    Ok(request_id) => request_id,
                    Err(_) => {
                        let mut reply = bot.send_message(msg.chat.id, "Invalid request id");
                        if let Some(thread_id) = msg.thread_id {
                            reply = reply.message_thread_id(thread_id);
                        }
                        reply.await?;
                        return Ok(());
                    }
                }
            },
            None => {
                let mut reply = bot.send_message(msg.chat.id, "Invalid command, use /reject_become_admin <request_id>");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        match self.db.reject_become_admin_request(&request_id).await {
            Ok(_) => {
                let mut reply = bot.send_message(msg.chat.id, "Admin rejected!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
            Err(e) => {
                log::error!("Error rejecting become admin request: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error rejecting become admin request!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
            }
        }

        Ok(())
    }

    async fn list_become_admin_requests(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
        log::trace!("Listing become admin requests: {:?}", msg);

        let requests = match self.db.get_become_admin_requests().await {
            Ok(requests) => requests,
            Err(e) => {
                log::error!("Error listing become admin requests: {:?}", e);
                let mut reply = bot.send_message(msg.chat.id, "Error listing become admin requests!");
                if let Some(thread_id) = msg.thread_id {
                    reply = reply.message_thread_id(thread_id);
                }
                reply.await?;
                return Ok(());
            }
        };

        let mut message_lines = vec!["Become admin requests:".to_string()];
        for request in requests {
            message_lines.push(format!("{:?}", request));
        }
        
        let mut reply = bot.send_message(msg.chat.id, message_lines.join("\n"));
        if let Some(thread_id) = msg.thread_id {
            reply = reply.message_thread_id(thread_id);
        }
        reply.await?;

        Ok(())
    }

async fn help(&self, bot: &Bot, msg: &Message) -> ResponseResult<()> {
    log::trace!("Help: {:?}", msg);

    let help = r#"
<b>🛠️ Admin Commands</b>
/list_admins — List all registered admins.
/remove_admin &lt;user_id&gt; — Remove an admin.
/make_superadmin &lt;user_id&gt; — Promote an admin to superadmin.
/become_admin — Request admin access.
/approve_become_admin &lt;request_id&gt; — Approve a request to become admin.
/reject_become_admin &lt;request_id&gt; — Reject a request.
/list_become_admin_requests — List all pending admin requests.

<b>✅ Whitelist Management</b>
/whitelist_group — Whitelist the current group.
/whitelist_thread — Whitelist the current thread.
/unwhitelist_group [group_id] — Remove a group from whitelist.
/unwhitelist_thread [group_id] [thread_id] — Remove a thread from whitelist.
/list_whitelisted_groups — Show all whitelisted groups.
/list_whitelisted_threads — Show whitelisted threads in this group.

<b>ℹ️ General</b>
/help — Show this help message.
"#;

    let mut reply = bot.send_message(msg.chat.id, help)
        .parse_mode(teloxide::types::ParseMode::Html);

    if let Some(thread_id) = msg.thread_id {
        reply = reply.message_thread_id(thread_id);
    }

    reply.await?;

    Ok(())
}
}
