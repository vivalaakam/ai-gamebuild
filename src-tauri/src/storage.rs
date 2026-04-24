use crate::model::{Project, ScriptUnit, StructUnit, InputAction, Tileset, Tilemap, Entity, EntityId};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::BTreeMap;
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
    entity_id TEXT NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (project_id, entity_id)
);

CREATE TABLE IF NOT EXISTS input_actions (
    project_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    label TEXT NOT NULL,
    key_code TEXT NOT NULL,
    event TEXT NOT NULL,
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
            .prepare("SELECT name, snapshot FROM projects WHERE id = ?1")
            .map_err(|err| err.to_string())?;
        let (name, bytes) = stmt
            .query_row(params![project_id], |row| {
                let name: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((name, bytes))
            })
            .map_err(|err| err.to_string())?;
        let mut project: Project = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
        project.id = project_id.to_string();
        project.name = name;
        self.apply_db_overrides(&mut project)?;
        Ok(project)
    }

    pub fn create_project(&self, name: &str) -> Result<Project, String> {
        let mut project = Project::demo();
        let project_id = uuid::Uuid::new_v4().to_string();
        project.id = project_id;
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


    pub fn load_scripts(&self, project_id: &str) -> Result<Vec<ScriptUnit>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT script_id, name, source, dependencies, bindings FROM scripts WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let source: String = row.get(2)?;
                let deps: String = row.get(3)?;
                let binds: String = row.get(4)?;
                let dependencies: Vec<String> = serde_json::from_str(&deps).unwrap_or_default();
                let bindings: std::collections::BTreeSet<String> = serde_json::from_str(&binds).unwrap_or_default();
                Ok(ScriptUnit { id, name, source, dependencies, bindings })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn load_structs(&self, project_id: &str) -> Result<Vec<StructUnit>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT struct_id, name, source FROM structs WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let source: String = row.get(2)?;
                Ok(StructUnit { id, name, source })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn load_input_actions(&self, project_id: &str) -> Result<Vec<InputAction>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT action_id, label, key_code FROM input_actions WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                let id: String = row.get(0)?;
                let label: String = row.get(1)?;
                let key_code: String = row.get(2)?;
                Ok(InputAction { id, label, key_code })
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn load_tileset(&self, project_id: &str) -> Result<Option<Tileset>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT metadata FROM tilesets WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let result = stmt.query_row(params![project_id], |row| {
            let meta: String = row.get(0)?;
            Ok(meta)
        });
        match result {
            Ok(meta) => {
                let tileset: Tileset = serde_json::from_str(&meta).map_err(|err| err.to_string())?;
                Ok(Some(tileset))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn load_tilemap(&self, project_id: &str) -> Result<Option<Tilemap>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM tilemaps WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let result = stmt.query_row(params![project_id], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        });
        match result {
            Ok(bytes) => {
                let tilemap: Tilemap = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
                Ok(Some(tilemap))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn load_entities(&self, project_id: &str) -> Result<BTreeMap<EntityId, Entity>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM entities WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                let data: String = row.get(0)?;
                let entity: Entity = serde_json::from_str(&data).map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
                Ok((entity.id, entity))
            })
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn load_runtime_state(&self, project_id: &str) -> Result<Option<Value>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM runtime_state WHERE project_id = ?1")
            .map_err(|err| err.to_string())?;
        let result = stmt.query_row(params![project_id], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        });
        match result {
            Ok(data) => {
                let state: Value = serde_json::from_str(&data).map_err(|err| err.to_string())?;
                Ok(Some(state))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    fn apply_db_overrides(&self, project: &mut Project) -> Result<(), String> {
        if let Ok(scripts) = self.load_scripts(&project.id) {
            if !scripts.is_empty() {
                project.scripts = scripts;
            }
        }
        if let Ok(structs) = self.load_structs(&project.id) {
            if !structs.is_empty() {
                project.structs = structs;
            }
        }
        if let Ok(actions) = self.load_input_actions(&project.id) {
            if !actions.is_empty() {
                project.input_actions = actions;
            }
        }
        if let Ok(Some(tileset)) = self.load_tileset(&project.id) {
            project.tileset = tileset;
        }
        if let Ok(Some(tilemap)) = self.load_tilemap(&project.id) {
            project.world.tilemap = tilemap;
        }
        if let Ok(entities) = self.load_entities(&project.id) {
            if !entities.is_empty() {
                project.world.entities = entities;
            }
        }
        if let Ok(Some(runtime_state)) = self.load_runtime_state(&project.id) {
            project.runtime_state = runtime_state;
        }
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
            .prepare("SELECT id, name, snapshot FROM projects ORDER BY updated_at DESC LIMIT 1")
            .map_err(|err| err.to_string())?;
        let result = stmt.query_row([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let bytes: Vec<u8> = row.get(2)?;
            Ok((id, name, bytes))
        });

        match result {
            Ok((id, name, bytes)) => {
                let mut project: Project = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
                project.id = id;
                project.name = name;
                self.apply_db_overrides(&mut project)?;
                Ok(project)
            }
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
