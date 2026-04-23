use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const VIRTUAL_WIDTH: u32 = 720;
pub const VIRTUAL_HEIGHT: u32 = 720;
pub const TILE_SIZE: u32 = 32;

pub type EntityId = u64;
pub type TileId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub scripts: Vec<ScriptUnit>,
    pub tileset: Tileset,
    pub world: World,
}

impl Project {
    pub fn demo() -> Self {
        Self {
            id: "default".into(),
            name: "Fantasy Console MVP".into(),
            scripts: vec![ScriptUnit::demo()],
            tileset: Tileset::demo(),
            world: World::demo(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptUnit {
    pub id: String,
    pub name: String,
    pub source: String,
    pub dependencies: Vec<String>,
    pub bindings: BTreeSet<String>,
}

impl ScriptUnit {
    pub fn demo() -> Self {
        let mut bindings = BTreeSet::new();
        bindings.insert("init".into());
        bindings.insert("input".into());
        bindings.insert("update".into());
        Self {
            id: "main".into(),
            name: "main.rhai".into(),
            dependencies: Vec::new(),
            bindings,
            source: r#"fn on_init(payload) {
    log("project initialized");
}

fn on_input(payload) {
    if is_just_pressed("paint") {
        set_tile(4, 4, 5);
        emit("painted", #{ x: 4, y: 4 });
    }

    if is_just_pressed("spawn") {
        let id = spawn_entity("slime", 7, 7);
        log("spawned entity");
    }
}

fn on_update(payload) {
    if is_pressed("right") {
        set_entity_pos(1, 12, 10);
    }
}"#
            .into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tileset {
    pub texture: String,
    pub columns: u32,
    pub tile_count: u32,
}

impl Tileset {
    pub fn demo() -> Self {
        Self {
            texture: "generated://debug-atlas".into(),
            columns: 8,
            tile_count: 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub tilemap: Tilemap,
    pub entities: BTreeMap<EntityId, Entity>,
    pub next_entity_id: EntityId,
    pub camera: Camera,
}

impl World {
    pub fn demo() -> Self {
        let mut entities = BTreeMap::new();
        entities.insert(
            1,
            Entity {
                id: 1,
                transform: Transform { x: 10, y: 10 },
                render: RenderComponent { tile_id: 2 },
                flags: EntityFlags {
                    visible: true,
                    blocking: true,
                },
                script: Some(ScriptBinding {
                    script_id: "main".into(),
                }),
                state: json!({ "kind": "hero" }),
            },
        );

        Self {
            tilemap: Tilemap::demo(48, 48),
            entities,
            next_entity_id: 2,
            camera: Camera { x: 0, y: 0 },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TileId>,
}

impl Tilemap {
    pub fn demo(width: u32, height: u32) -> Self {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let tile = if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    3
                } else if (x + y) % 9 == 0 {
                    4
                } else {
                    1
                };
                tiles.push(tile);
            }
        }
        Self {
            width,
            height,
            tiles,
        }
    }

    pub fn get(&self, x: i64, y: i64) -> Option<TileId> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        self.tiles
            .get((y as u32 * self.width + x as u32) as usize)
            .copied()
    }

    pub fn set(&mut self, x: i64, y: i64, tile_id: TileId) -> bool {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return false;
        }
        let index = (y as u32 * self.width + x as u32) as usize;
        self.tiles[index] = tile_id;
        true
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Camera {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub transform: Transform,
    pub render: RenderComponent,
    pub flags: EntityFlags,
    pub script: Option<ScriptBinding>,
    pub state: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RenderComponent {
    pub tile_id: TileId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntityFlags {
    pub visible: bool,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptBinding {
    pub script_id: String,
}
