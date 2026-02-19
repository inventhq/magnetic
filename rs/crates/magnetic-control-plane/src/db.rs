use libsql::{params, Connection, Database};

use crate::error::AppError;

pub struct Db {
    inner: Database,
}

// ── Row types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub tier: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct App {
    pub id: String,
    pub name: Option<String>,
    pub user_id: String,
    pub node_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Node {
    pub id: String,
    pub ip: String,
    pub port: i64,
    pub region: String,
    pub app_count: i64,
    pub max_apps: i64,
    pub status: String,
    pub civo_instance_id: Option<String>,
    pub created_at: String,
}

// ── Connection helper ───────────────────────────────────────────────

impl Db {
    pub async fn connect_remote(url: &str, token: &str) -> Result<Self, AppError> {
        let db = libsql::Builder::new_remote(url.to_string(), token.to_string())
            .build()
            .await
            .map_err(|e| AppError::Database(format!("turso connect: {}", e)))?;
        Ok(Self { inner: db })
    }

    pub async fn connect_local(path: &str) -> Result<Self, AppError> {
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .map_err(|e| AppError::Database(format!("local db: {}", e)))?;
        Ok(Self { inner: db })
    }

    fn conn(&self) -> Result<Connection, AppError> {
        self.inner.connect().map_err(|e| AppError::Database(e.to_string()))
    }

    pub async fn init_schema(&self) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id         TEXT PRIMARY KEY,
                email      TEXT UNIQUE NOT NULL,
                tier       TEXT NOT NULL DEFAULT 'free',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            (),
        )
        .await?;
        c.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                key_hash   TEXT PRIMARY KEY,
                user_id    TEXT NOT NULL,
                name       TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            )",
            (),
        )
        .await?;
        c.execute(
            "CREATE TABLE IF NOT EXISTS apps (
                id         TEXT PRIMARY KEY,
                name       TEXT UNIQUE,
                user_id    TEXT NOT NULL,
                node_id    TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id),
                FOREIGN KEY (node_id) REFERENCES nodes(id)
            )",
            (),
        )
        .await?;
        c.execute(
            "CREATE TABLE IF NOT EXISTS nodes (
                id               TEXT PRIMARY KEY,
                ip               TEXT NOT NULL,
                port             INTEGER NOT NULL DEFAULT 3003,
                region           TEXT NOT NULL DEFAULT 'LON1',
                app_count        INTEGER NOT NULL DEFAULT 0,
                max_apps         INTEGER NOT NULL DEFAULT 300,
                status           TEXT NOT NULL DEFAULT 'active',
                civo_instance_id TEXT,
                created_at       TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            (),
        )
        .await?;
        // Indexes for fast lookups
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id)",
            (),
        )
        .await?;
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_apps_user ON apps(user_id)",
            (),
        )
        .await?;
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_apps_name ON apps(name) WHERE name IS NOT NULL",
            (),
        )
        .await?;
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_apps_node ON apps(node_id)",
            (),
        )
        .await?;
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes(status)",
            (),
        )
        .await?;
        Ok(())
    }

    // ── Users ───────────────────────────────────────────────────────

    pub async fn create_user(&self, id: &str, email: &str) -> Result<User, AppError> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO users (id, email) VALUES (?1, ?2)",
            params![id, email],
        )
        .await?;
        Ok(User {
            id: id.to_string(),
            email: email.to_string(),
            tier: "free".into(),
            created_at: String::new(), // filled by DB default
        })
    }

    pub async fn get_user(&self, id: &str) -> Result<Option<User>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query("SELECT id, email, tier, created_at FROM users WHERE id = ?1", params![id])
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(User {
                id: row.get::<String>(0)?,
                email: row.get::<String>(1)?,
                tier: row.get::<String>(2)?,
                created_at: row.get::<String>(3)?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, email, tier, created_at FROM users WHERE email = ?1",
                params![email],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(User {
                id: row.get::<String>(0)?,
                email: row.get::<String>(1)?,
                tier: row.get::<String>(2)?,
                created_at: row.get::<String>(3)?,
            })),
            None => Ok(None),
        }
    }

    // ── API Keys ────────────────────────────────────────────────────

    pub async fn store_api_key(
        &self,
        key_hash: &str,
        user_id: &str,
        name: &str,
    ) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "INSERT INTO api_keys (key_hash, user_id, name) VALUES (?1, ?2, ?3)",
            params![key_hash, user_id, name],
        )
        .await?;
        Ok(())
    }

    pub async fn get_user_by_key_hash(&self, key_hash: &str) -> Result<Option<User>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT u.id, u.email, u.tier, u.created_at
                 FROM api_keys k JOIN users u ON k.user_id = u.id
                 WHERE k.key_hash = ?1",
                params![key_hash],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(User {
                id: row.get::<String>(0)?,
                email: row.get::<String>(1)?,
                tier: row.get::<String>(2)?,
                created_at: row.get::<String>(3)?,
            })),
            None => Ok(None),
        }
    }

    // ── Apps ────────────────────────────────────────────────────────

    pub async fn create_app(
        &self,
        id: &str,
        name: Option<&str>,
        user_id: &str,
        node_id: &str,
    ) -> Result<App, AppError> {
        let c = self.conn()?;
        let name_val = name.unwrap_or("");
        c.execute(
            "INSERT INTO apps (id, name, user_id, node_id)
             VALUES (?1, NULLIF(?2, ''), ?3, ?4)",
            params![id, name_val, user_id, node_id],
        )
        .await?;
        Ok(App {
            id: id.to_string(),
            name: name.map(String::from),
            user_id: user_id.to_string(),
            node_id: node_id.to_string(),
            created_at: String::new(),
            updated_at: String::new(),
        })
    }

    pub async fn get_app(&self, id: &str) -> Result<Option<App>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, name, user_id, node_id, created_at, updated_at
                 FROM apps WHERE id = ?1",
                params![id],
            )
            .await?;
        Self::read_app_row(&mut rows).await
    }

    pub async fn get_app_by_name(&self, name: &str) -> Result<Option<App>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, name, user_id, node_id, created_at, updated_at
                 FROM apps WHERE name = ?1",
                params![name],
            )
            .await?;
        Self::read_app_row(&mut rows).await
    }

    pub async fn list_apps_for_user(&self, user_id: &str) -> Result<Vec<App>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, name, user_id, node_id, created_at, updated_at
                 FROM apps WHERE user_id = ?1 ORDER BY created_at DESC",
                params![user_id],
            )
            .await?;
        let mut apps = Vec::new();
        while let Some(row) = rows.next().await? {
            apps.push(App {
                id: row.get::<String>(0)?,
                name: row.get::<String>(1).ok(),
                user_id: row.get::<String>(2)?,
                node_id: row.get::<String>(3)?,
                created_at: row.get::<String>(4)?,
                updated_at: row.get::<String>(5)?,
            });
        }
        Ok(apps)
    }

    pub async fn update_app_node(&self, app_id: &str, node_id: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "UPDATE apps SET node_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![node_id, app_id],
        )
        .await?;
        Ok(())
    }

    pub async fn delete_app(&self, app_id: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute("DELETE FROM apps WHERE id = ?1", params![app_id]).await?;
        Ok(())
    }

    pub async fn count_apps_for_user(&self, user_id: &str) -> Result<i64, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT COUNT(*) FROM apps WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(row.get::<i64>(0)?),
            None => Ok(0),
        }
    }

    /// Resolve a subdomain to its node URL. Checks both app ID and vanity name.
    pub async fn resolve_subdomain(&self, subdomain: &str) -> Result<Option<(App, Node)>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT a.id, a.name, a.user_id, a.node_id, a.created_at, a.updated_at,
                        n.id, n.ip, n.port, n.region, n.app_count, n.max_apps, n.status,
                        n.civo_instance_id, n.created_at
                 FROM apps a JOIN nodes n ON a.node_id = n.id
                 WHERE a.id = ?1 OR a.name = ?1
                 LIMIT 1",
                params![subdomain],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let app = App {
                    id: row.get::<String>(0)?,
                    name: row.get::<String>(1).ok(),
                    user_id: row.get::<String>(2)?,
                    node_id: row.get::<String>(3)?,
                    created_at: row.get::<String>(4)?,
                    updated_at: row.get::<String>(5)?,
                };
                let node = Node {
                    id: row.get::<String>(6)?,
                    ip: row.get::<String>(7)?,
                    port: row.get::<i64>(8)?,
                    region: row.get::<String>(9)?,
                    app_count: row.get::<i64>(10)?,
                    max_apps: row.get::<i64>(11)?,
                    status: row.get::<String>(12)?,
                    civo_instance_id: row.get::<String>(13).ok(),
                    created_at: row.get::<String>(14)?,
                };
                Ok(Some((app, node)))
            }
            None => Ok(None),
        }
    }

    async fn read_app_row(rows: &mut libsql::Rows) -> Result<Option<App>, AppError> {
        match rows.next().await? {
            Some(row) => Ok(Some(App {
                id: row.get::<String>(0)?,
                name: row.get::<String>(1).ok(),
                user_id: row.get::<String>(2)?,
                node_id: row.get::<String>(3)?,
                created_at: row.get::<String>(4)?,
                updated_at: row.get::<String>(5)?,
            })),
            None => Ok(None),
        }
    }

    // ── Nodes ───────────────────────────────────────────────────────

    pub async fn create_node(
        &self,
        id: &str,
        ip: &str,
        port: i64,
        region: &str,
        civo_instance_id: Option<&str>,
    ) -> Result<Node, AppError> {
        let c = self.conn()?;
        let civo_id = civo_instance_id.unwrap_or("");
        c.execute(
            "INSERT INTO nodes (id, ip, port, region, civo_instance_id)
             VALUES (?1, ?2, ?3, ?4, NULLIF(?5, ''))",
            params![id, ip, port, region, civo_id],
        )
        .await?;
        Ok(Node {
            id: id.to_string(),
            ip: ip.to_string(),
            port,
            region: region.to_string(),
            app_count: 0,
            max_apps: 300,
            status: "active".into(),
            civo_instance_id: civo_instance_id.map(String::from),
            created_at: String::new(),
        })
    }

    pub async fn list_nodes(&self) -> Result<Vec<Node>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, ip, port, region, app_count, max_apps, status,
                        civo_instance_id, created_at
                 FROM nodes ORDER BY app_count ASC",
                (),
            )
            .await?;
        let mut nodes = Vec::new();
        while let Some(row) = rows.next().await? {
            nodes.push(Node {
                id: row.get::<String>(0)?,
                ip: row.get::<String>(1)?,
                port: row.get::<i64>(2)?,
                region: row.get::<String>(3)?,
                app_count: row.get::<i64>(4)?,
                max_apps: row.get::<i64>(5)?,
                status: row.get::<String>(6)?,
                civo_instance_id: row.get::<String>(7).ok(),
                created_at: row.get::<String>(8)?,
            });
        }
        Ok(nodes)
    }

    /// Pick the active node with the lowest load that has capacity.
    pub async fn select_node(&self) -> Result<Option<Node>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, ip, port, region, app_count, max_apps, status,
                        civo_instance_id, created_at
                 FROM nodes
                 WHERE status = 'active' AND app_count < max_apps
                 ORDER BY app_count ASC
                 LIMIT 1",
                (),
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(Node {
                id: row.get::<String>(0)?,
                ip: row.get::<String>(1)?,
                port: row.get::<i64>(2)?,
                region: row.get::<String>(3)?,
                app_count: row.get::<i64>(4)?,
                max_apps: row.get::<i64>(5)?,
                status: row.get::<String>(6)?,
                civo_instance_id: row.get::<String>(7).ok(),
                created_at: row.get::<String>(8)?,
            })),
            None => Ok(None),
        }
    }

    pub async fn increment_node_app_count(&self, node_id: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "UPDATE nodes SET app_count = app_count + 1 WHERE id = ?1",
            params![node_id],
        )
        .await?;
        Ok(())
    }

    pub async fn decrement_node_app_count(&self, node_id: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "UPDATE nodes SET app_count = MAX(0, app_count - 1) WHERE id = ?1",
            params![node_id],
        )
        .await?;
        Ok(())
    }

    pub async fn update_node_status(&self, node_id: &str, status: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute(
            "UPDATE nodes SET status = ?1 WHERE id = ?2",
            params![status, node_id],
        )
        .await?;
        Ok(())
    }

    pub async fn delete_node(&self, node_id: &str) -> Result<(), AppError> {
        let c = self.conn()?;
        c.execute("DELETE FROM nodes WHERE id = ?1", params![node_id]).await?;
        Ok(())
    }

    /// Get all apps on a given node.
    pub async fn list_apps_on_node(&self, node_id: &str) -> Result<Vec<App>, AppError> {
        let c = self.conn()?;
        let mut rows = c
            .query(
                "SELECT id, name, user_id, node_id, created_at, updated_at
                 FROM apps WHERE node_id = ?1",
                params![node_id],
            )
            .await?;
        let mut apps = Vec::new();
        while let Some(row) = rows.next().await? {
            apps.push(App {
                id: row.get::<String>(0)?,
                name: row.get::<String>(1).ok(),
                user_id: row.get::<String>(2)?,
                node_id: row.get::<String>(3)?,
                created_at: row.get::<String>(4)?,
                updated_at: row.get::<String>(5)?,
            });
        }
        Ok(apps)
    }
}
