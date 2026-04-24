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

CREATE TABLE IF NOT EXISTS structs (
    project_id TEXT NOT NULL,
    struct_id TEXT NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY (project_id, struct_id)
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

CREATE TABLE IF NOT EXISTS input_actions (
    project_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    label TEXT NOT NULL,
    key_code TEXT NOT NULL,
    PRIMARY KEY (project_id, action_id)
);

CREATE TABLE IF NOT EXISTS runtime_state (
    project_id TEXT PRIMARY KEY,
    data TEXT NOT NULL
);
"#;

pub struct ProjectStore {
    conn: Connection,
}

impl ProjectStore {
        pub fn list_projects(&self) -> Result<Vec<(String, String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, updated_at FROM projects ORDER BY updated_at DESC")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn load_project(&self, project_id: &str) -> Result<Project, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT snapshot FROM projects WHERE id = ?1")
            .map_err(|err| err.to_string())?;
        let bytes: Vec<u8> = stmt
            .query_row(params![project_id], |row| row.get(0))
            .map_err(|err| err.to_string())?;
        let mut project: Project = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
        project.id = project_id.to_string();
        Ok(project)
    }

    pub fn create_project(&self, project_id: &str, name: &str) -> Result<Project, String> {
        let mut project = Project::demo();
        project.id = project_id.to_string();
        project.name = name.to_string();
        self.save_snapshot(&project)?;
        Ok(project)
    }

    pub fn delete_project(&self, project_id: &str) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM scripts WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM structs WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM tilesets WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM tilemaps WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM entities WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM input_actions WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM runtime_state WHERE project_id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![project_id])
            .map_err(|err| err.to_string())?;
        tx.commit().map_err(|err| err.to_string())?;
        Ok(())
    }

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

        for unit in &project.structs {
            tx.execute(
                "INSERT INTO structs (project_id, struct_id, name, source)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_id, struct_id) DO UPDATE SET
                    name = excluded.name,
                    source = excluded.source",
                params![project.id, unit.id, unit.name, unit.source],
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

        for action in &project.input_actions {
            tx.execute(
                "INSERT INTO input_actions (project_id, action_id, label, key_code)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_id, action_id) DO UPDATE SET
                    label = excluded.label,
                    key_code = excluded.key_code",
                params![project.id, action.id, action.label, action.key_code],
            )
            .map_err(|err| err.to_string())?;
        }

        tx.execute(
            "INSERT INTO runtime_state (project_id, data)
             VALUES (?1, ?2)
             ON CONFLICT(project_id) DO UPDATE SET data = excluded.data",
            params![
                project.id,
                serde_json::to_string(&project.runtime_state).map_err(|err| err.to_string())?
            ],
        )
        .map_err(|err| err.to_string())?;

        tx.commit().map_err(|err| err.to_string())?;
        Ok(snapshot)
    }
}
