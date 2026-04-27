use ai_rpg_engine::{EngineConfig, GameEngine, Project};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: usize = 720;
const WINDOW_HEIGHT: usize = 720;

fn main() -> Result<(), String> {
    let project = Project::demo();

    let mut engine = GameEngine::new(EngineConfig {
        width: WINDOW_WIDTH as u32,
        height: WINDOW_HEIGHT as u32,
        project,
        palette: None,
    })?;

    let mut window = Window::new(
        "AI RPG World Viewer",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    window.limit_update_rate(Some(Duration::from_micros(16_600)));

    let mut last_frame = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let pressed = collect_pressed_keys(&mut window);
        let delta = last_frame.elapsed().as_secs_f64().min(0.1);
        last_frame = Instant::now();

        let frame = engine.step(pressed, delta);
        let rgba = engine.render_frame(&frame);

        // Convert RGBA bytes → XRGB u32 for minifb
        let buffer: Vec<u32> = rgba
            .chunks_exact(4)
            .map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32))
            .collect();

        window
            .update_with_buffer(&buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .map_err(|e| e.to_string())?;

        for line in frame.logs.iter().take(4) {
            println!("{line}");
        }
    }

    Ok(())
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
