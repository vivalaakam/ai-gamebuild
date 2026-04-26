use minifb::{Key, KeyRepeat, Window, WindowOptions};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const DEFAULT_URL: &str = "http://127.0.0.1:3001/jsonrpc";
const WINDOW_WIDTH: usize = 720;
const WINDOW_HEIGHT: usize = 720;
const PALETTE: [u32; 8] = [
    0x101820, 0x243c2f, 0xf2c14e, 0x1b263b, 0x3a7d44, 0xe76f51, 0x2a9d8f, 0xe9c46a,
];

#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct VisibleTile {
    tile_id: u32,
    screen_x: i32,
    screen_y: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct VisibleEntity {
    id: u64,
    tile_id: u32,
    screen_x: i32,
    screen_y: i32,
    is_active: bool,
    opacity: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct FrameView {
    virtual_width: u32,
    virtual_height: u32,
    tile_size: u32,
    visible_tiles: Vec<VisibleTile>,
    visible_entities: Vec<VisibleEntity>,
    logs: Vec<String>,
}

struct JsonRpcClient {
    url: String,
    next_id: u64,
}

impl JsonRpcClient {
    fn new(url: String) -> Self {
        Self { url, next_id: 1 }
    }

    fn call<T: DeserializeOwned>(&mut self, method: &str, params: Value) -> Result<T, String> {
        let id = self.next_id;
        self.next_id += 1;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        };
        let response = ureq::post(&self.url)
            .send_json(serde_json::to_value(request).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
            .into_json::<JsonRpcResponse>()
            .map_err(|e| e.to_string())?;

        if let Some(error) = response.error {
            return Err(format!("jsonrpc error {}: {}", error.code, error.message));
        }
        let result = response.result.ok_or_else(|| "missing result".to_string())?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }
}

fn main() -> Result<(), String> {
    let url = resolve_url();
    let mut client = JsonRpcClient::new(url);
    let mut window = Window::new(
        "AI RPG World Viewer",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    window.limit_update_rate(Some(Duration::from_micros(16_600)));

    let mut buffer = vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT];
    let mut last_frame = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let pressed = collect_pressed_keys(&mut window);
        let delta = last_frame.elapsed().as_secs_f64().min(0.1);
        last_frame = Instant::now();

        let frame: FrameView = client.call(
            "run_frame",
            json!({
                "pressed_keys": pressed,
                "delta": delta,
            }),
        )?;

        render_frame(&frame, &mut buffer);
        window
            .update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .map_err(|e| e.to_string())?;

        if !frame.logs.is_empty() {
            for line in frame.logs.iter().take(4) {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn resolve_url() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--url" {
            if let Some(value) = args.next() {
                return value;
            }
        }
    }
    DEFAULT_URL.to_string()
}

fn collect_pressed_keys(window: &mut Window) -> Vec<String> {
    let mut pressed = HashSet::new();
    for key in window.get_keys_pressed(KeyRepeat::Yes) {
        if let Some(code) = map_key_code(key) {
            pressed.insert(code.to_string());
        }
    }
    for key in window.get_keys() {
        if let Some(code) = map_key_code(key) {
            pressed.insert(code.to_string());
        }
    }
    pressed.into_iter().collect()
}

fn map_key_code(key: Key) -> Option<&'static str> {
    match key {
        Key::Up => Some("ArrowUp"),
        Key::Down => Some("ArrowDown"),
        Key::Left => Some("ArrowLeft"),
        Key::Right => Some("ArrowRight"),
        Key::Space => Some("Space"),
        Key::Enter => Some("Enter"),
        Key::Tab => Some("Tab"),
        _ => None,
    }
}

fn render_frame(frame: &FrameView, buffer: &mut [u32]) {
    let width = WINDOW_WIDTH as i32;
    let height = WINDOW_HEIGHT as i32;
    let scale_x = WINDOW_WIDTH as f32 / frame.virtual_width as f32;
    let scale_y = WINDOW_HEIGHT as f32 / frame.virtual_height as f32;
    let scale = scale_x.min(scale_y);

    fill(buffer, PALETTE[0]);

    for tile in &frame.visible_tiles {
        let color = PALETTE[(tile.tile_id as usize) % PALETTE.len()];
        let x = (tile.screen_x as f32 * scale) as i32;
        let y = (tile.screen_y as f32 * scale) as i32;
        let size = (frame.tile_size as f32 * scale) as i32;
        draw_rect(buffer, width, height, x, y, size, size, color);
        draw_rect_outline(buffer, width, height, x, y, size, size, 0x243c2f);
    }

    for entity in &frame.visible_entities {
        let color = PALETTE[(entity.tile_id as usize) % PALETTE.len()];
        let x = (entity.screen_x as f32 * scale) as i32;
        let y = (entity.screen_y as f32 * scale) as i32;
        let size = (frame.tile_size as f32 * scale) as i32;
        let inset = (4.0 * scale).max(1.0) as i32;
        let inner = size - inset * 2;
        let color = if entity.opacity < 0.9 {
            fade_color(color, 0.5)
        } else {
            color
        };
        draw_rect(buffer, width, height, x + inset, y + inset, inner, inner, color);
        if entity.is_active {
            draw_rect_outline(buffer, width, height, x + inset, y + inset, inner, inner, 0xfff7e8);
        }
    }
}

fn fill(buffer: &mut [u32], color: u32) {
    for pixel in buffer.iter_mut() {
        *pixel = color;
    }
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
