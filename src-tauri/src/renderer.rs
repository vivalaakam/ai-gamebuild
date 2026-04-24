use crate::model::{Camera, EntityId, Project, TileId, TILE_SIZE, VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DrawCommand {
    Clear,
    Tile { tile_id: TileId, x: i32, y: i32 },
    Entity { entity_id: EntityId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleTile {
    pub tile_id: TileId,
    pub map_x: i32,
    pub map_y: i32,
    pub screen_x: i32,
    pub screen_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleEntity {
    pub id: EntityId,
    pub tile_id: TileId,
    pub screen_x: i32,
    pub screen_y: i32,
    pub is_active: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameView {
    pub virtual_width: u32,
    pub virtual_height: u32,
    pub tile_size: u32,
    pub camera: Camera,
    pub visible_tiles: Vec<VisibleTile>,
    pub visible_entities: Vec<VisibleEntity>,
    pub draw_commands: Vec<DrawCommand>,
    pub logs: Vec<String>,
}

pub fn build_frame(
    project: &Project,
    draw_commands: Vec<DrawCommand>,
    logs: Vec<String>,
) -> FrameView {
    let camera = project.world.camera;
    let tile_size = TILE_SIZE as i32;
    let cols = (VIRTUAL_WIDTH / TILE_SIZE + 2) as i32;
    let rows = (VIRTUAL_HEIGHT / TILE_SIZE + 2) as i32;
    let start_x = camera.x.div_euclid(tile_size);
    let start_y = camera.y.div_euclid(tile_size);

    let mut visible_tiles = Vec::new();
    for y in start_y..start_y + rows {
        for x in start_x..start_x + cols {
            if let Some(tile_id) = project.world.tilemap.get(x as i64, y as i64) {
                visible_tiles.push(VisibleTile {
                    tile_id,
                    map_x: x,
                    map_y: y,
                    screen_x: x * tile_size - camera.x,
                    screen_y: y * tile_size - camera.y,
                });
            }
        }
    }

    let visible_entities = project
        .runtime_state
        .get("active_entity_id")
        .and_then(|value| value.as_u64())
        .map(|active_id| {
            project
                .world
                .entities
                .values()
                .filter(|entity| entity.flags.visible)
                .map(|entity| {
                    let is_active = entity.id == active_id;
                    VisibleEntity {
                        id: entity.id,
                        tile_id: entity.render.tile_id,
                        screen_x: entity.transform.x * tile_size - camera.x,
                        screen_y: entity.transform.y * tile_size - camera.y,
                        is_active,
                        opacity: if is_active { 1.0 } else { 0.5 },
                    }
                })
                .collect()
        })
        .unwrap_or_else(|| {
            project
                .world
                .entities
                .values()
                .filter(|entity| entity.flags.visible)
                .map(|entity| VisibleEntity {
                    id: entity.id,
                    tile_id: entity.render.tile_id,
                    screen_x: entity.transform.x * tile_size - camera.x,
                    screen_y: entity.transform.y * tile_size - camera.y,
                    is_active: false,
                    opacity: 0.5,
                })
                .collect()
        });

    FrameView {
        virtual_width: VIRTUAL_WIDTH,
        virtual_height: VIRTUAL_HEIGHT,
        tile_size: TILE_SIZE,
        camera,
        visible_tiles,
        visible_entities,
        draw_commands,
        logs,
    }
}
