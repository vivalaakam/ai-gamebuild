use ai_rpg_engine::{
    EngineConfig, FrameView, GameEngine, Project, RawInput, VIRTUAL_HEIGHT, VIRTUAL_WIDTH,
};
use crate::storage::ProjectStore;

pub use ai_rpg_engine::ValidationResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct Runtime {
    engine: GameEngine,
    store: ProjectStore,
}

impl Runtime {
    pub fn open(db_path: PathBuf, default_project_id: Option<String>) -> Result<Self, String> {
        let store = ProjectStore::open(db_path)?;
        let project = if let Some(project_id) = default_project_id {
            store.load_project(&project_id)
        } else {
            store.load_or_seed()
        }?;

        let engine = GameEngine::new(EngineConfig {
            width: VIRTUAL_WIDTH,
            height: VIRTUAL_HEIGHT,
            project,
            palette: None,
        })?;

        Ok(Self { engine, store })
    }

    pub fn project(&self) -> Project {
        self.engine.project()
    }

    pub fn frame(&mut self, raw_input: RawInput, delta: f64) -> FrameView {
        self.engine.step(raw_input.pressed_keys, delta)
    }

    pub fn emit(&mut self, name: String, payload: serde_json::Value) {
        self.engine.emit(name, payload);
    }

    pub fn update_script(&mut self, script_id: String, source: String) -> Result<Project, String> {
        self.engine.update_script(script_id, source)
    }

    pub fn validate_script(&self, source: String) -> ValidationResult {
        self.engine.validate_script(source)
    }

    pub fn create_script(&mut self) -> Result<Project, String> {
        self.engine.create_script()
    }

    pub fn create_struct(&mut self) -> Project {
        self.engine.create_struct()
    }

    pub fn update_struct(&mut self, struct_id: String, source: String) -> Result<Project, String> {
        self.engine.update_struct(struct_id, source)
    }

    pub fn update_input_action(
        &mut self,
        action_id: String,
        key_code: String,
    ) -> Result<Project, String> {
        self.engine.update_input_action(action_id, key_code)
    }

    pub fn reset_input_actions(&mut self) -> Project {
        self.engine.reset_input_actions()
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectInfo>, String> {
        let rows = self.store.list_projects()?;
        Ok(rows
            .into_iter()
            .map(|(id, name, updated_at)| ProjectInfo { id, name, updated_at })
            .collect())
    }

    pub fn switch_project(&mut self, project_id: String) -> Result<Project, String> {
        let project = self.store.load_project(&project_id)?;
        self.engine.reload_project(project)?;
        Ok(self.engine.project())
    }

    pub fn create_project(&self, name: String) -> Result<Project, String> {
        self.store.create_project(&name)
    }

    pub fn delete_project(&self, project_id: String) -> Result<(), String> {
        self.store.delete_project(&project_id)
    }

    pub fn save(&self) -> Result<SaveResult, String> {
        let project = self.engine.project();
        let snapshot = self.store.save_snapshot(&project)?;
        Ok(SaveResult { snapshot_bytes: snapshot.len() })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResult {
    pub snapshot_bytes: usize,
}
