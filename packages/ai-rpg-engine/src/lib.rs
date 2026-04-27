mod events;
mod input;
mod scripting;
mod renderer;
mod engine;

pub mod model;

pub use engine::{EngineConfig, GameEngine, ValidationResult};
pub use renderer::{FrameView, DEFAULT_PALETTE};
pub use input::RawInput;
pub use scripting::{validate_source, validate_project_source};
pub use model::{
    Camera, Entity, EntityFlags, EntityId, InputAction, Project, RenderComponent, ScriptBinding,
    ScriptUnit, StructUnit, TileId, Tilemap, Tileset, Transform, World, TILE_SIZE, VIRTUAL_HEIGHT,
    VIRTUAL_WIDTH,
};
