use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DB {
    db: Arc<Mutex<Connection>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Admin {
    pub user_id: u64,
    pub name: Option<String>,
    pub added_by: Option<u64>,
    pub added_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct WhitelistedGroup {
    pub group_id: i64,
    pub group_name: Option<String>,
    pub added_by: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct WhitelistedThread {
    pub thread_id: i32,
    pub group_id: i64,
    pub group_name: Option<String>,
    pub thread_name: Option<String>,
    pub added_by: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
}

impl Admin {
    pub fn is_superadmin(&self) -> bool {
        self.added_by.is_none()
    }
}

impl DB {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let db = Connection::open(path)?;
        Ok(Self { db: Arc::new(Mutex::new(db)) })
    }

    pub async fn migrate(&self) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;

        conn.execute_batch(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/init.sql")))?;

        log::info!("Database migration applied successfully.");

        Ok(())
    }

    pub async fn add_admin(
        &self, 
        user_id: u64, 
        added_by: u64,
        name: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "INSERT INTO admins (user_id, name, added_by) VALUES (?1, ?2, ?3)",
            params![user_id, name, added_by],
        )?;

        Ok(())
    }

    pub async fn add_whitelisted_group(
        &self, 
        group_id: i64, 
        added_by: u64,
        group_name: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "INSERT INTO whitelisted_groups (group_id, group_name, added_by) VALUES (?1, ?2, ?3)",
            params![group_id, group_name, added_by],
        )?;

        Ok(())
    }

    pub async fn add_whitelisted_thread(
        &self, 
        thread_id: i32, 
        group_id: i64, 
        added_by: u64,
        group_name: Option<&str>,
        thread_name: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "INSERT INTO whitelisted_threads (thread_id, group_id, added_by, group_name, thread_name) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![thread_id, group_id, added_by, group_name, thread_name],
        )?;

        Ok(())
    }

    pub async fn remove_whitelisted_group(
        &self, 
        group_id: i64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "DELETE FROM whitelisted_groups WHERE group_id = ?1",
            params![group_id],
        )?;

        Ok(())
    }

    pub async fn remove_whitelisted_thread(
        &self, 
        thread_id: i32,
        group_id: i64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "DELETE FROM whitelisted_threads WHERE thread_id = ?1 AND group_id = ?2",
            params![thread_id, group_id],
        )?;

        Ok(())
    }

    pub async fn remove_admin(
        &self, 
        user_id: u64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "DELETE FROM admins WHERE user_id = ?1",
            params![user_id],
        )?;

        Ok(())
    }

    pub async fn remove_admin_with_traversal(
        &self, 
        user_id: u64,
        remover_id: u64,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().await;

        let mut admin = match self.get_admin(user_id).await? {
            Some(admin) => admin,
            None => return Ok(false),
        };

        // Traverse the adder chain until find the remover
        while let Some(adder) = admin.added_by {
            if adder == remover_id {
                conn.execute(
                    "DELETE FROM admins WHERE user_id = ?1",
                    params![user_id],
                )?;
                return Ok(true);
            }

            admin = match self.get_admin(adder).await? {
                Some(admin) => admin,
                None => return Ok(false),
            };
        }
        
        Ok(false)
    }

    pub async fn make_superadmin(
        &self, 
        user_id: u64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().await;
        conn.execute(
            "UPDATE admins SET added_by = NULL WHERE user_id = ?1",
            params![user_id],
        )?;
        Ok(())
    }

    pub async fn get_whitelisted_threads(
        &self, 
        group_id: i64,
    ) -> Result<Vec<WhitelistedThread>, rusqlite::Error> {
        let conn = self.db.lock().await;

        let mut stmt = conn.prepare(
            "SELECT thread_id, group_id, group_name, thread_name, added_by, created_at FROM whitelisted_threads WHERE group_id = ?1",
        )?;
        let mut rows = stmt.query(params![group_id])?;
        
        let mut threads = Vec::new();
        
        while let Some(row) = rows.next()? {
            threads.push(WhitelistedThread {
                thread_id: row.get(0)?,
                group_id: row.get(1)?,
                group_name: row.get(2)?,
                thread_name: row.get(3)?,
                added_by: row.get(4)?,
                created_at: row.get(5)?,
            });
        }
        
        Ok(threads)
    }

    pub async fn get_whitelisted_groups(&self) -> Result<Vec<WhitelistedGroup>, rusqlite::Error> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT group_id, group_name, added_by, created_at FROM whitelisted_groups")?;
        
        let mut groups = Vec::new();
        
        while let Some(row) = stmt.query([])?.next()? {
            groups.push(WhitelistedGroup {
                group_id: row.get(0)?,
                group_name: row.get(1)?,
                added_by: row.get(2)?,
                created_at: row.get(3)?,
            });
        }
        
        Ok(groups)
    }

    pub async fn get_admin(&self, user_id: u64) -> Result<Option<Admin>, rusqlite::Error> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT user_id, name, added_by, added_at FROM admins WHERE user_id = ?1")?;
        let mut rows = stmt.query(params![user_id])?;

        if let Some(row) = rows.next()? {
            return Ok(Some(Admin {
                user_id: row.get(0)?,
                name: row.get(1)?,
                added_by: row.get(2)?,
                added_at: row.get(3)?,
            }));
        }

        Ok(None)
    }

    pub async fn get_admins(&self) -> Result<Vec<Admin>, rusqlite::Error> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT user_id, name, added_by, added_at FROM admins")?;
        
        let mut admins = Vec::new();
        
        while let Some(row) = stmt.query([])?.next()? {
            admins.push(Admin {
                user_id: row.get(0)?,
                name: row.get(1)?,
                added_by: row.get(2)?,
                added_at: row.get(3)?,
            });
        }
        
        Ok(admins)
    }

    pub async fn is_group_whitelisted(&self, group_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM whitelisted_groups WHERE group_id = ?1")?;
        let mut rows = stmt.query(params![group_id])?;

        if rows.next()?.is_some() {
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn is_thread_whitelisted(&self, thread_id: i32, group_id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().await;
        let mut stmt = conn.prepare("SELECT 1 FROM whitelisted_threads WHERE thread_id = ?1 AND group_id = ?2")?;
        let mut rows = stmt.query(params![thread_id, group_id])?;

        if rows.next()?.is_some() {
            return Ok(true);
        }

        Ok(false)
    }
}

