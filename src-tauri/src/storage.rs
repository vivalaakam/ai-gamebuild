use crate::model::Project;
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    snapshot BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS scripts (
    project_id TEXT NOT NULL,
    script_id TEXT NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    dependencies TEXT NOT NULL,
    bindings TEXT NOT NULL,
    PRIMARY KEY (project_id, script_id)
);

CREATE TABLE IF NOT EXISTS tilesets (
    project_id TEXT PRIMARY KEY,
    metadata TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tilemaps (
    project_id TEXT PRIMARY KEY,
    data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS entities (
    project_id TEXT NOT NULL,
    entity_id INTEGER NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (project_id, entity_id)
);
"#;

pub struct ProjectStore {
    conn: Connection,
}

impl ProjectStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|err| err.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|err| err.to_string())?;
        Ok(Self { conn })
    }

    pub fn load_or_seed(&self) -> Result<Project, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT snapshot FROM projects WHERE id = ?1")
            .map_err(|err| err.to_string())?;
        let existing: Result<Vec<u8>, _> = stmt.query_row(params!["default"], |row| row.get(0));

        match existing {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|err| err.to_string()),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let project = Project::demo();
                self.save_snapshot(&project)?;
                Ok(project)
            }
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn save_snapshot(&self, project: &Project) -> Result<Vec<u8>, String> {
        let snapshot = serde_json::to_vec(project).map_err(|err| err.to_string())?;
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|err| err.to_string())?;
        tx.execute(
            "INSERT INTO projects (id, name, snapshot, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                snapshot = excluded.snapshot,
                updated_at = CURRENT_TIMESTAMP",
            params![project.id, project.name, snapshot],
        )
        .map_err(|err| err.to_string())?;

        for script in &project.scripts {
            tx.execute(
                "INSERT INTO scripts (project_id, script_id, name, source, dependencies, bindings)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(project_id, script_id) DO UPDATE SET
                    name = excluded.name,
                    source = excluded.source,
                    dependencies = excluded.dependencies,
                    bindings = excluded.bindings",
                params![
                    project.id,
                    script.id,
                    script.name,
                    script.source,
                    serde_json::to_string(&script.dependencies).map_err(|err| err.to_string())?,
                    serde_json::to_string(&script.bindings).map_err(|err| err.to_string())?,
                ],
            )
            .map_err(|err| err.to_string())?;
        }

        tx.execute(
            "INSERT INTO tilesets (project_id, metadata)
             VALUES (?1, ?2)
             ON CONFLICT(project_id) DO UPDATE SET metadata = excluded.metadata",
            params![
                project.id,
                serde_json::to_string(&project.tileset).map_err(|err| err.to_string())?
            ],
        )
        .map_err(|err| err.to_string())?;

        tx.execute(
            "INSERT INTO tilemaps (project_id, data)
             VALUES (?1, ?2)
             ON CONFLICT(project_id) DO UPDATE SET data = excluded.data",
            params![
                project.id,
                serde_json::to_vec(&project.world.tilemap).map_err(|err| err.to_string())?
            ],
        )
        .map_err(|err| err.to_string())?;

        for entity in project.world.entities.values() {
            tx.execute(
                "INSERT INTO entities (project_id, entity_id, data)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(project_id, entity_id) DO UPDATE SET data = excluded.data",
                params![
                    project.id,
                    entity.id as i64,
                    serde_json::to_string(entity).map_err(|err| err.to_string())?
                ],
            )
            .map_err(|err| err.to_string())?;
        }

        tx.commit().map_err(|err| err.to_string())?;
        Ok(snapshot)
    }
}
