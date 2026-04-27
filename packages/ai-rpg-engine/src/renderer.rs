use crate::model::{Camera, EntityId, Project, TileId, TILE_SIZE, VIRTUAL_HEIGHT, VIRTUAL_WIDTH};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PALETTE: [u32; 8] = [
    0x101820, 0x243c2f, 0xf2c14e, 0x1b263b, 0x3a7d44, 0xe76f51, 0x2a9d8f, 0xe9c46a,
];

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

/// Renders a FrameView to a raw RGBA byte buffer (4 bytes per pixel, row-major).
pub fn render_to_rgba(frame: &FrameView, width: u32, height: u32, palette: &[u32]) -> Vec<u8> {
    let w = width as i32;
    let h = height as i32;
    let scale_x = width as f32 / frame.virtual_width as f32;
    let scale_y = height as f32 / frame.virtual_height as f32;
    let scale = scale_x.min(scale_y);

    let bg = palette.first().copied().unwrap_or(0);
    let mut buffer = vec![bg; (width * height) as usize];

    for tile in &frame.visible_tiles {
        let color = palette_color(palette, tile.tile_id as usize);
        let x = (tile.screen_x as f32 * scale) as i32;
        let y = (tile.screen_y as f32 * scale) as i32;
        let size = (frame.tile_size as f32 * scale) as i32;
        draw_rect(&mut buffer, w, h, x, y, size, size, color);
        let outline = palette.get(1).copied().unwrap_or(0x243c2f);
        draw_rect_outline(&mut buffer, w, h, x, y, size, size, outline);
    }

    for entity in &frame.visible_entities {
        let base_color = palette_color(palette, entity.tile_id as usize);
        let color = if entity.opacity < 0.9 {
            fade_color(base_color, 0.5)
        } else {
            base_color
        };
        let x = (entity.screen_x as f32 * scale) as i32;
        let y = (entity.screen_y as f32 * scale) as i32;
        let size = (frame.tile_size as f32 * scale) as i32;
        let inset = (4.0 * scale).max(1.0) as i32;
        let inner = size - inset * 2;
        draw_rect(&mut buffer, w, h, x + inset, y + inset, inner, inner, color);
        if entity.is_active {
            draw_rect_outline(&mut buffer, w, h, x + inset, y + inset, inner, inner, 0xfff7e8);
        }
    }

    // Convert XRGB u32 → RGBA bytes
    buffer
        .iter()
        .flat_map(|&xrgb| {
            let r = ((xrgb >> 16) & 0xff) as u8;
            let g = ((xrgb >> 8) & 0xff) as u8;
            let b = (xrgb & 0xff) as u8;
            [r, g, b, 255u8]
        })
        .collect()
}

fn palette_color(palette: &[u32], index: usize) -> u32 {
    if palette.is_empty() {
        return 0;
    }
    palette[index % palette.len()]
}

fn draw_rect(buffer: &mut [u32], width: i32, height: i32, x: i32, y: i32, w: i32, h: i32, color: u32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(width);
    let y1 = (y + h).min(height);
    for yy in y0..y1 {
        let row = (yy * width) as usize;
        for xx in x0..x1 {
            buffer[row + xx as usize] = color;
        }
    }
}

fn draw_rect_outline(
    buffer: &mut [u32],
    width: i32,
    height: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u32,
) {
    draw_rect(buffer, width, height, x, y, w, 1, color);
    draw_rect(buffer, width, height, x, y + h - 1, w, 1, color);
    draw_rect(buffer, width, height, x, y, 1, h, color);
    draw_rect(buffer, width, height, x + w - 1, y, 1, h, color);
}

fn fade_color(color: u32, factor: f32) -> u32 {
    let r = ((color >> 16) & 0xff) as f32;
    let g = ((color >> 8) & 0xff) as f32;
    let b = (color & 0xff) as f32;
    let r = (r * factor).min(255.0) as u32;
    let g = (g * factor).min(255.0) as u32;
    let b = (b * factor).min(255.0) as u32;
    (r << 16) | (g << 8) | b
}
