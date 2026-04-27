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
    #[serde(default = "StructUnit::demo_structs")]
    pub structs: Vec<StructUnit>,
    #[serde(default = "InputAction::defaults")]
    pub input_actions: Vec<InputAction>,
    #[serde(default = "default_runtime_state_placeholder")]
    pub runtime_state: Value,
    pub tileset: Tileset,
    pub world: World,
}

impl Project {
    pub fn demo() -> Self {
        let world = World::demo();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Fantasy Console MVP".into(),
            scripts: ScriptUnit::demo_scripts(),
            structs: StructUnit::demo_structs(),
            input_actions: InputAction::defaults(),
            runtime_state: default_runtime_state(&world),
            tileset: Tileset::demo(),
            world,
        }
    }

    pub fn normalize_demo_scripts(&mut self) {
        if self.input_actions.is_empty() {
            self.input_actions = InputAction::defaults();
        }
        if self.structs.is_empty() {
            self.structs = StructUnit::demo_structs();
        }
        if self.runtime_state.is_null() {
            self.runtime_state = default_runtime_state(&self.world);
        }
        ensure_runtime_state(&mut self.runtime_state, &self.world);
        sync_world_from_runtime_state(&mut self.world, &self.runtime_state);
        let builtin_ids: BTreeSet<String> = ScriptUnit::builtin_libraries()
            .into_iter()
            .map(|unit| unit.id)
            .collect();
        self.scripts
            .retain(|script| !(script.bindings.is_empty() && builtin_ids.contains(&script.id)));
    }
}

fn default_runtime_state_placeholder() -> Value {
    json!({})
}

fn default_runtime_state(world: &World) -> Value {
    let player_positions = world
        .entities
        .values()
        .map(|entity| {
            (
                entity.id.to_string(),
                json!({
                    "x": entity.transform.x,
                    "y": entity.transform.y,
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();

    json!({
        "active_entity_id": world.entities.keys().next().copied().unwrap_or(0),
        "player_positions": Value::Object(player_positions),
    })
}

fn ensure_runtime_state(runtime_state: &mut Value, world: &World) {
    if !runtime_state.is_object() {
        *runtime_state = default_runtime_state(world);
        return;
    }

    let Some(state) = runtime_state.as_object_mut() else {
        return;
    };

    state
        .entry("active_entity_id")
        .or_insert_with(|| json!(world.entities.keys().next().copied().unwrap_or(0)));

    let positions = state
        .entry("player_positions")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    if !positions.is_object() {
        *positions = Value::Object(serde_json::Map::new());
    }

    if let Some(map) = positions.as_object_mut() {
        for entity in world.entities.values() {
            map.entry(entity.id.to_string()).or_insert_with(|| {
                json!({
                    "x": entity.transform.x,
                    "y": entity.transform.y,
                })
            });
        }
    }
}

fn sync_world_from_runtime_state(world: &mut World, runtime_state: &Value) {
    let Some(positions) = runtime_state
        .get("player_positions")
        .and_then(Value::as_object)
    else {
        return;
    };

    for entity in world.entities.values_mut() {
        let Some(position) = positions
            .get(&entity.id.to_string())
            .and_then(Value::as_object)
        else {
            continue;
        };
        let Some(x) = position.get("x").and_then(Value::as_i64) else {
            continue;
        };
        let Some(y) = position.get("y").and_then(Value::as_i64) else {
            continue;
        };
        entity.transform.x = x as i32;
        entity.transform.y = y as i32;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAction {
    pub id: String,
    pub label: String,
    pub key_code: String,
}

impl InputAction {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self::new("up", "ArrowUp", "ArrowUp"),
            Self::new("down", "ArrowDown", "ArrowDown"),
            Self::new("left", "ArrowLeft", "ArrowLeft"),
            Self::new("right", "ArrowRight", "ArrowRight"),
            Self::new("paint", "Paint", "Space"),
            Self::new("spawn", "Spawn", "Enter"),
            Self::new("next", "Next", "Tab"),
        ]
    }

    fn new(id: impl Into<String>, label: impl Into<String>, key_code: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            key_code: key_code.into(),
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
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        source: impl Into<String>,
        bindings: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let bindings = bindings.into_iter().map(Into::into).collect();
        Self {
            id: id.into(),
            name: name.into(),
            dependencies: Vec::new(),
            bindings,
            source: source.into(),
        }
    }

    pub fn demo_scripts() -> Vec<Self> {
        let mut scripts = Vec::new();
        scripts.extend([
            Self::new(
                "init",
                "init.rhai",
                r#"fn on_init(payload) {
    log("project initialized");
}"#,
                ["init"],
            ),
            Self::new(
                "input",
                "input.rhai",
                r#"fn on_input(payload) {
    if !payload.pressed {
        return;
    }

    if payload.action == "up" {
        move_active(0, -1);
    }

    if payload.action == "down" {
        move_active(0, 1);
    }

    if payload.action == "left" {
        move_active(-1, 0);
    }

    if payload.action == "right" {
        move_active(1, 0);
    }

    if payload.action == "next" {
        next_active();
    }

    if is_just_pressed(payload, "paint") {
        set_tile(4, 4, 5);
        emit("painted", #{ x: 4, y: 4 });
    }

    if is_just_pressed(payload, "spawn") {
        let id = spawn_entity("slime", 7, 7);
        log("spawned entity");
    }
}"#,
                ["input"],
            ),
            Self::new(
                "update",
                "update.rhai",
                r#"fn on_update(payload) {
}"#,
                ["update"],
            ),
        ]);
        scripts
    }

    pub fn builtin_libraries() -> Vec<Self> {
        vec![
            Self::new(
                "entities",
                "entities.rhai",
                r#"fn spawn_entity(kind, x, y) {
    let actor = make_actor(kind, 2);
    let id = entity_spawn_raw(actor.kind, x, y, actor.tile_id, false);
    let positions = get_player_positions();
    positions[id.to_string()] = #{ x: x, y: y };
    state_set("player_positions", positions);

    if get_active() == 0 {
        state_set("active_entity_id", id);
    }

    return id;
}

fn set_entity_pos(id, x, y) {
    let ok = entity_set_pos_raw(id, x, y);

    if ok {
        let positions = get_player_positions();
        positions[id.to_string()] = #{ x: x, y: y };
        state_set("player_positions", positions);
    }

    return ok;
}"#,
                Vec::<String>::new(),
            ),
            Self::new(
                "state",
                "state.rhai",
                r#"fn get_active() {
    return state_get("active_entity_id");
}

fn get_player_positions() {
    let positions = state_get("player_positions");
    if type_of(positions) == "map" {
        return positions;
    }

    return #{};
}

fn get_player_pos(id) {
    let positions = get_player_positions();
    let key = id.to_string();
    if positions.contains(key) {
        return positions[key];
    }

    let entity = get_entity(id);
    return #{ x: entity.x, y: entity.y };
}

fn move_active(dx, dy) {
    let active_id = get_active();
    let pos = get_player_pos(active_id);
    return set_entity_pos(active_id, pos.x + dx, pos.y + dy);
}

fn next_active() {
    let next = entity_next_id(get_active());
    state_set("active_entity_id", next);
    return next;
}"#,
                Vec::<String>::new(),
            ),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructUnit {
    pub id: String,
    pub name: String,
    pub source: String,
}

impl StructUnit {
    pub fn new(id: impl Into<String>, name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source: source.into(),
        }
    }

    pub fn demo_structs() -> Vec<Self> {
        vec![Self::new(
            "actor",
            "actor.rhai",
            r#"fn make_actor(kind, tile_id) {
    return #{
        kind: kind,
        tile_id: tile_id,
        hp: 1,
        visible: true,
        blocking: false
    };
}"#,
        )]
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
        entities.insert(1, demo_entity(1, 10, 10, 2, "hero"));
        entities.insert(2, demo_entity(2, 13, 10, 5, "mage"));
        entities.insert(3, demo_entity(3, 16, 10, 6, "rogue"));

        Self {
            tilemap: Tilemap::demo(48, 48),
            entities,
            next_entity_id: 4,
            camera: Camera { x: 0, y: 0 },
        }
    }
}

fn demo_entity(id: EntityId, x: i32, y: i32, tile_id: TileId, kind: &str) -> Entity {
    Entity {
        id,
        transform: Transform { x, y },
        render: RenderComponent { tile_id },
        flags: EntityFlags {
            visible: true,
            blocking: true,
        },
        script: Some(ScriptBinding {
            script_id: "input".into(),
        }),
        state: json!({ "kind": kind }),
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
