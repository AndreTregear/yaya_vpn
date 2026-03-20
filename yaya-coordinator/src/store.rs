use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: String,
    pub public_key: String,
    pub mesh_ip: String,
    pub endpoints: Vec<String>,
    pub is_exit: bool,
    pub last_seen: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterPeer {
    pub public_key: String,
    pub endpoints: Vec<String>,
    #[serde(default)]
    pub is_exit: bool,
}

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS peers (
                id TEXT PRIMARY KEY,
                public_key TEXT UNIQUE NOT NULL,
                mesh_ip TEXT UNIQUE NOT NULL,
                endpoints TEXT NOT NULL DEFAULT '[]',
                is_exit INTEGER NOT NULL DEFAULT 0,
                last_seen TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS relays (
                id TEXT PRIMARY KEY,
                address TEXT NOT NULL,
                region TEXT,
                last_seen TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn register_peer(&self, req: &RegisterPeer) -> Result<Peer> {
        let conn = self.conn.lock().unwrap();

        // Check if peer already registered
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM peers WHERE public_key = ?1",
                [&req.public_key],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            // Update endpoints and last_seen
            let endpoints_json = serde_json::to_string(&req.endpoints)?;
            conn.execute(
                "UPDATE peers SET endpoints = ?1, is_exit = ?2, last_seen = datetime('now') WHERE id = ?3",
                rusqlite::params![endpoints_json, req.is_exit, id],
            )?;
            return self.get_peer_by_id(&id);
        }

        // Allocate mesh IP
        let mesh_ip = self.allocate_ip(&conn)?;
        let id = uuid::Uuid::new_v4().to_string();
        let endpoints_json = serde_json::to_string(&req.endpoints)?;

        conn.execute(
            "INSERT INTO peers (id, public_key, mesh_ip, endpoints, is_exit, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            rusqlite::params![id, req.public_key, mesh_ip, endpoints_json, req.is_exit],
        )?;

        Ok(Peer {
            id,
            public_key: req.public_key.clone(),
            mesh_ip,
            endpoints: req.endpoints.clone(),
            is_exit: req.is_exit,
            last_seen: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_peers(&self) -> Result<Vec<Peer>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, public_key, mesh_ip, endpoints, is_exit, last_seen FROM peers",
        )?;

        let peers = stmt
            .query_map([], |row| {
                let endpoints_str: String = row.get(3)?;
                let endpoints: Vec<String> =
                    serde_json::from_str(&endpoints_str).unwrap_or_default();
                Ok(Peer {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    mesh_ip: row.get(2)?,
                    endpoints,
                    is_exit: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(peers)
    }

    pub fn get_peer_by_id(&self, id: &str) -> Result<Peer> {
        let conn = self.conn.lock().unwrap();
        let peer = conn.query_row(
            "SELECT id, public_key, mesh_ip, endpoints, is_exit, last_seen FROM peers WHERE id = ?1",
            [id],
            |row| {
                let endpoints_str: String = row.get(3)?;
                let endpoints: Vec<String> =
                    serde_json::from_str(&endpoints_str).unwrap_or_default();
                Ok(Peer {
                    id: row.get(0)?,
                    public_key: row.get(1)?,
                    mesh_ip: row.get(2)?,
                    endpoints,
                    is_exit: row.get(4)?,
                    last_seen: row.get(5)?,
                })
            },
        )?;
        Ok(peer)
    }

    pub fn remove_peer(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM peers WHERE id = ?1", [id])?;
        Ok(rows > 0)
    }

    fn allocate_ip(&self, conn: &Connection) -> Result<String> {
        // Find the next available IP in 100.64.0.0/10
        let max_offset: Option<u32> = conn
            .query_row(
                "SELECT MAX(CAST(SUBSTR(mesh_ip, 9) AS INTEGER)) FROM peers WHERE mesh_ip LIKE '100.64.%'",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let next_offset = max_offset.unwrap_or(1) + 1;

        // Convert offset to IP within 100.64.0.0/10
        let third_octet = (next_offset / 256) as u8;
        let fourth_octet = (next_offset % 256) as u8;

        if third_octet > 63 {
            anyhow::bail!("Mesh IP space exhausted");
        }

        Ok(format!("100.64.{third_octet}.{fourth_octet}"))
    }
}
