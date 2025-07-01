use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use sqlx::Error;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct DB {
    db: Arc<SqlitePool>,
}

#[allow(unused)]
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Admin {
    pub user_id: i64,
    pub name: Option<String>,
    pub added_by: Option<i64>,
    pub added_at: Option<DateTime<Utc>>,
}

#[allow(unused)]
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct WhitelistedGroup {
    pub group_id: i64,
    pub group_name: Option<String>,
    pub added_by: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[allow(unused)]
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct WhitelistedThread {
    pub thread_id: i32,
    pub group_id: i64,
    pub group_name: Option<String>,
    pub thread_name: Option<String>,
    pub added_by: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[allow(unused)]
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct BecomeAdminRequest {
    pub request_id: String,
    pub user_id: i64,
    pub user_name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub pending: Option<bool>,
    pub accepted: Option<bool>,
}

impl Admin {
    pub fn is_superadmin(&self) -> bool {
        self.added_by.is_none()
    }
}

impl DB {
    pub async fn new(path: &str) -> Result<Self, Error> {
        let db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(path)
            .await?;
        Ok(Self { db: Arc::new(db) })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        MIGRATOR.run(&*self.db).await?;
        log::info!("Migrations applied successfully.");
        Ok(())
    }

    pub async fn add_admin(&self, user_id: i64, added_by: i64, name: Option<&str>) -> Result<(), Error> {
        sqlx::query("INSERT INTO admins (user_id, name, added_by) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(name)
            .bind(added_by)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn add_whitelisted_group(&self, group_id: i64, added_by: i64, group_name: Option<&str>) -> Result<(), Error> {
        sqlx::query("INSERT INTO whitelisted_groups (group_id, group_name, added_by) VALUES (?, ?, ?)")
            .bind(group_id)
            .bind(group_name)
            .bind(added_by)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn add_whitelisted_thread(
        &self,
        thread_id: i32,
        group_id: i64,
        added_by: i64,
        group_name: Option<&str>,
        thread_name: Option<&str>,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO whitelisted_threads (thread_id, group_id, added_by, group_name, thread_name) VALUES (?, ?, ?, ?, ?)")
            .bind(thread_id)
            .bind(group_id)
            .bind(added_by)
            .bind(group_name)
            .bind(thread_name)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn remove_whitelisted_group(&self, group_id: i64) -> Result<(), Error> {
        sqlx::query("DELETE FROM whitelisted_groups WHERE group_id = ?")
            .bind(group_id)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn remove_whitelisted_thread(&self, thread_id: i32, group_id: i64) -> Result<(), Error> {
        sqlx::query("DELETE FROM whitelisted_threads WHERE thread_id = ? AND group_id = ?")
            .bind(thread_id)
            .bind(group_id)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn remove_admin(&self, user_id: i64) -> Result<(), Error> {
        sqlx::query("DELETE FROM admins WHERE user_id = ?")
            .bind(user_id )
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn remove_admin_with_traversal(&self, user_id: i64, remover_id: i64) -> Result<bool, Error> {
        let mut admin = match self.get_admin(user_id).await? {
            Some(a) => a,
            None => return Ok(false),
        };

        while let Some(adder) = admin.added_by {
            if adder == remover_id {
                self.remove_admin(user_id).await?;
                return Ok(true);
            }
            admin = match self.get_admin(adder).await? {
                Some(a) => a,
                None => return Ok(false),
            };
        }

        Ok(false)
    }

    pub async fn make_superadmin(&self, user_id: i64) -> Result<(), Error> {
        sqlx::query("UPDATE admins SET added_by = NULL WHERE user_id = ?")
            .bind(user_id)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn get_whitelisted_threads(&self, group_id: i64) -> Result<Vec<WhitelistedThread>, Error> {
        let threads = sqlx::query_as::<_, WhitelistedThread>(
            "SELECT thread_id, group_id, group_name, thread_name, added_by, created_at FROM whitelisted_threads WHERE group_id = ?",
        )
        .bind(group_id)
        .fetch_all(&*self.db)
        .await?;

        Ok(threads)
    }

    pub async fn get_whitelisted_groups(&self) -> Result<Vec<WhitelistedGroup>, Error> {
        let groups = sqlx::query_as::<_, WhitelistedGroup>(
            "SELECT group_id, group_name, added_by, created_at FROM whitelisted_groups",
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(groups)
    }

    pub async fn get_admin(&self, user_id: i64) -> Result<Option<Admin>, Error> {
        let admin = sqlx::query_as::<_, Admin>(
            "SELECT user_id, name, added_by, added_at FROM admins WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&*self.db)
        .await?;

        Ok(admin)
    }

    pub async fn get_admins(&self) -> Result<Vec<Admin>, Error> {
        let admins = sqlx::query_as::<_, Admin>(
            "SELECT user_id, name, added_by, added_at FROM admins",
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(admins)
    }

    pub async fn is_group_whitelisted(&self, group_id: i64) -> Result<bool, Error> {
        let exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM whitelisted_groups WHERE group_id = ?")
            .bind(group_id)
            .fetch_optional(&*self.db)
            .await?;

        Ok(exists.is_some())
    }

    pub async fn is_thread_whitelisted(&self, thread_id: i32, group_id: i64) -> Result<bool, Error> {
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM whitelisted_threads WHERE thread_id = ? AND group_id = ?",
        )
        .bind(thread_id)
        .bind(group_id)
        .fetch_optional(&*self.db)
        .await?;

        Ok(exists.is_some())
    }

    pub async fn create_become_admin_request(&self, user_id: i64, user_name: Option<&str>) -> Result<Option<String>, Error> {
        let request_id = Uuid::now_v7();

        // Check if there is a pending request for this user
        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT request_id FROM become_admin_requests WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&*self.db)
        .await?;

        if exists.is_some() {
            return Ok(None)
        }

        sqlx::query("INSERT INTO become_admin_requests (request_id, user_id, user_name) VALUES (?, ?, ?)")
            .bind(request_id.to_string())
            .bind(user_id)
            .bind(user_name)
            .execute(&*self.db)
            .await?;
        
        Ok(Some(request_id.to_string()))
    }

    pub async fn get_become_admin_requests(&self) -> Result<Vec<BecomeAdminRequest>, Error> {
        let requests = sqlx::query_as::<_, BecomeAdminRequest>(
            "SELECT request_id, user_id, user_name, created_at, pending, accepted FROM become_admin_requests",
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(requests)
    }

    pub async fn approve_become_admin_request(&self, request_id: &str, admin_id: i64) -> Result<(), Error> {
        // Create transaction, approve the request and create the admin
        // if the request is not pending, return an error
        let request = sqlx::query_as::<_, BecomeAdminRequest>(
            "SELECT request_id, user_id, user_name, created_at, pending, accepted FROM become_admin_requests WHERE request_id = ?",
        )
            .bind(request_id)
            .fetch_optional(&*self.db)
            .await?;

        if request.is_none() {
            return Err(Error::RowNotFound);
        }

        let request = request.unwrap();

        if request.pending.unwrap_or(false) {
            sqlx::query("UPDATE become_admin_requests SET accepted = TRUE WHERE request_id = ?")
                .bind(request_id)
                .execute(&*self.db)
                .await?;

            self.add_admin(request.user_id, admin_id, request.user_name.as_deref()).await?;

            return Ok(())
        }

        Err(Error::RowNotFound)
    }

    pub async fn reject_become_admin_request(&self, request_id: &str) -> Result<(), Error> {
        // Create transaction, reject the request and delete the request
        // if the request is not pending, return an error
        let request = sqlx::query_as::<_, BecomeAdminRequest>(
            "SELECT request_id, user_id, user_name, created_at, pending, accepted FROM become_admin_requests WHERE request_id = ?",
        )
            .bind(request_id)
            .fetch_optional(&*self.db)
            .await?;

        if request.is_none() {
            return Err(Error::RowNotFound);
        }

        let request = request.unwrap();

        if request.pending.unwrap_or(false) {
            sqlx::query("UPDATE become_admin_requests SET pending = FALSE WHERE request_id = ?")
                .bind(request_id)
                .execute(&*self.db)
                .await?;

            sqlx::query("DELETE FROM become_admin_requests WHERE request_id = ?")
                .bind(request_id)
                .execute(&*self.db)
                .await?;

            return Ok(())
        }

        Err(Error::RowNotFound)
    }
}
