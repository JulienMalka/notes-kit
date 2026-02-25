use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone)]
pub struct ServerUser {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub assigned_levels: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ServerUser {
    pub fn to_core_user(&self) -> notes_kit_core::models::User {
        notes_kit_core::models::User {
            id: self.id.clone(),
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            assigned_levels: self.assigned_levels.clone(),
            session_hash: self.password_hash.clone(),
        }
    }
}

#[derive(Clone)]
pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                display_name TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_levels (
                user_id TEXT NOT NULL,
                level_name TEXT NOT NULL,
                granted_at INTEGER NOT NULL,
                PRIMARY KEY (user_id, level_name),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_user_levels_user ON user_levels(user_id);
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn build_server_user(&self, row: UserRow) -> Result<ServerUser, sqlx::Error> {
        let levels: Vec<String> = sqlx::query_scalar(
            "SELECT level_name FROM user_levels WHERE user_id = ? ORDER BY level_name",
        )
        .bind(&row.id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ServerUser {
            id: row.id,
            email: row.email,
            password_hash: row.password_hash,
            display_name: row.display_name,
            assigned_levels: levels,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn get_user(&self, user_id: &str) -> Result<Option<ServerUser>, sqlx::Error> {
        self.fetch_user_by("id", user_id).await
    }

    pub async fn get_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<ServerUser>, sqlx::Error> {
        self.fetch_user_by("email", email).await
    }

    async fn fetch_user_by(
        &self,
        column: &str,
        value: &str,
    ) -> Result<Option<ServerUser>, sqlx::Error> {
        let query = format!(
            "SELECT id, email, password_hash, display_name, created_at, updated_at \
             FROM users WHERE {column} = ?"
        );
        let row = sqlx::query_as::<_, UserRow>(&query)
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(self.build_server_user(r).await?)),
            None => Ok(None),
        }
    }

    pub async fn verify_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<ServerUser>, Box<dyn std::error::Error + Send + Sync>> {
        let Some(user) = self.get_user_by_email(email).await? else {
            return Ok(None);
        };
        let parsed_hash = PasswordHash::new(&user.password_hash)?;
        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        display_name: Option<String>,
        assigned_levels: Vec<String>,
    ) -> Result<ServerUser, Box<dyn std::error::Error + Send + Sync>> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        let now = chrono::Utc::now().timestamp();
        let user_id = email.to_string();

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, display_name, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&user_id)
        .bind(email)
        .bind(&password_hash)
        .bind(&display_name)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        for level in &assigned_levels {
            sqlx::query(
                "INSERT INTO user_levels (user_id, level_name, granted_at) VALUES (?, ?, ?)",
            )
            .bind(&user_id)
            .bind(level)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(ServerUser {
            id: user_id,
            email: email.to_string(),
            password_hash,
            display_name,
            assigned_levels,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn ensure_admin(
        &self,
        admin: &super::authz_config::AdminUserConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&self.pool)
                .await?;
        if count == 0 {
            self.create_user(
                &admin.email,
                &admin.password,
                Some("Admin".to_string()),
                admin.assigned_levels.clone(),
            )
            .await?;
            eprintln!("[auth] Created initial admin user: {}", admin.email);
        }
        Ok(())
    }

    pub async fn list_users(&self) -> Result<Vec<ServerUser>, sqlx::Error> {
        let rows = sqlx::query_as::<_, UserRow>(
            "SELECT id, email, password_hash, display_name, created_at, updated_at \
             FROM users ORDER BY email",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            users.push(self.build_server_user(row).await?);
        }
        Ok(users)
    }
}

#[derive(FromRow)]
struct UserRow {
    id: String,
    email: String,
    password_hash: String,
    display_name: Option<String>,
    created_at: i64,
    updated_at: i64,
}
