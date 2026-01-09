use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent, TrayIcon},
    Manager, PhysicalPosition, Emitter, AppHandle,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::io::Cursor;
use once_cell::sync::Lazy;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_notification::NotificationExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};

// Image rendering imports
use image::{ImageBuffer, Rgba, RgbaImage};
use rusttype::{Font, Scale};

// Store tray icon reference for updating
static TRAY_ICON: Lazy<Mutex<Option<TrayIcon>>> = Lazy::new(|| Mutex::new(None));

// Store active timer state for shortcut toggle
static ACTIVE_TIMER_STATE: Lazy<Mutex<Option<TimeEntry>>> = Lazy::new(|| Mutex::new(None));

// Store first project ID for quick-start via shortcut
static FIRST_PROJECT_ID: Lazy<Mutex<Option<i32>>> = Lazy::new(|| Mutex::new(None));

// Cache the last rendered timer text to avoid flickering on redundant updates
static LAST_TIMER_TEXT: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

// API types
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Client {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_f64")]
    pub hourly_rate: Option<f64>,
}

// Helper to deserialize hourly_rate which might be string, number, or null
fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(n.as_f64()),
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
                Ok(None)
            } else {
                // Be lenient - if it's not a valid number, just return None
                Ok(trimmed.parse::<f64>().ok())
            }
        }
        Some(_) => Ok(None),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserSettings {
    pub show_live_revenue: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub client: Option<Client>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeEntry {
    pub id: i32,
    pub project_id: i32,
    pub project: Option<Project>,
    pub description: Option<String>,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub resumed_at: Option<String>,
    pub duration_minutes: Option<f64>,
    pub is_billable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateEntryData {
    pub project_id: i32,
    pub description: Option<String>,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub is_billable: bool,
}

const API_BASE: &str = "http://localhost:8000/api/widget";
const WIDGET_SECRET: &str = "freshtracks-widget-dev-secret";

// WebSocket configuration for Laravel Reverb
const REVERB_HOST: &str = "localhost";
const REVERB_PORT: u16 = 8080;
const REVERB_APP_KEY: &str = "u2oi0cwsi7cmbmnkteku";
const WIDGET_USER_ID: i32 = 1; // Same as Laravel widget user

fn get_http_client() -> reqwest::Client {
    reqwest::Client::new()
}

// WebSocket connection for real-time sync
async fn connect_websocket(app: AppHandle) {
    let ws_url = format!("ws://{}:{}/app/{}?protocol=7&client=rust&version=1.0",
        REVERB_HOST, REVERB_PORT, REVERB_APP_KEY);

    println!("Connecting to WebSocket: {}", ws_url);

    loop {
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                println!("WebSocket connected successfully");
                let (mut write, mut read) = ws_stream.split();

                // Subscribe to the timers channel for this user (Pusher protocol)
                let subscribe_msg = serde_json::json!({
                    "event": "pusher:subscribe",
                    "data": {
                        "channel": format!("timers.{}", WIDGET_USER_ID)
                    }
                });

                if let Err(e) = write.send(Message::Text(subscribe_msg.to_string().into())).await {
                    eprintln!("Failed to subscribe to channel: {}", e);
                    continue;
                }
                println!("Subscribed to timers.{} channel", WIDGET_USER_ID);

                // Process incoming messages
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            handle_websocket_message(&app, &text);
                        }
                        Ok(Message::Ping(data)) => {
                            // Respond to ping with pong
                            let _ = write.send(Message::Pong(data)).await;
                        }
                        Ok(Message::Close(_)) => {
                            println!("WebSocket closed by server");
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to connect to WebSocket: {}", e);
            }
        }

        // Wait before reconnecting
        println!("Reconnecting in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

// Handle incoming WebSocket messages (Pusher protocol)
fn handle_websocket_message(app: &AppHandle, text: &str) {
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(event) = data.get("event").and_then(|e| e.as_str()) {
            match event {
                "timer.started" => {
                    println!("Received timer.started event");
                    if let Some(entry_data) = data.get("data") {
                        // Parse the data (may be nested JSON string)
                        let entry_json = if entry_data.is_string() {
                            serde_json::from_str::<serde_json::Value>(entry_data.as_str().unwrap_or("{}")).ok()
                        } else {
                            Some(entry_data.clone())
                        };

                        if let Some(entry) = entry_json.and_then(|d| d.get("entry").cloned()) {
                            let _ = app.emit("ws-timer-started", entry);
                        }
                    }
                }
                "timer.stopped" => {
                    println!("Received timer.stopped event");
                    if let Some(entry_data) = data.get("data") {
                        let entry_json = if entry_data.is_string() {
                            serde_json::from_str::<serde_json::Value>(entry_data.as_str().unwrap_or("{}")).ok()
                        } else {
                            Some(entry_data.clone())
                        };

                        if let Some(entry) = entry_json.and_then(|d| d.get("entry").cloned()) {
                            let _ = app.emit("ws-timer-stopped", entry);
                        }
                    }
                }
                "timer.deleted" => {
                    println!("Received timer.deleted event");
                    if let Some(entry_data) = data.get("data") {
                        let entry_json = if entry_data.is_string() {
                            serde_json::from_str::<serde_json::Value>(entry_data.as_str().unwrap_or("{}")).ok()
                        } else {
                            Some(entry_data.clone())
                        };

                        if let Some(entry_id) = entry_json.and_then(|d| d.get("entry_id").cloned()) {
                            let _ = app.emit("ws-timer-deleted", entry_id);
                        }
                    }
                }
                "pusher:subscription_succeeded" => {
                    println!("Successfully subscribed to channel");
                }
                "pusher:connection_established" => {
                    println!("Pusher connection established");
                }
                "pusher:pong" | "pusher:ping" => {
                    // Heartbeat, ignore
                }
                _ => {
                    if !event.starts_with("pusher:") {
                        println!("Unknown event: {}", event);
                    }
                }
            }
        }
    }
}

// Icons (embedded at compile time)
const ICON_INACTIVE: &[u8] = include_bytes!("../icons/tray-icon.png");
// For active state, we'll use the same icon with tooltip indicating state
// Future enhancement: create a green-tinted version for active state
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-icon.png");

// Embedded font for timer text rendering
const FONT_DATA: &[u8] = include_bytes!("../fonts/RobotoMono-Regular.ttf");

// Helper function to check if a point is inside a rounded rectangle
fn is_inside_rounded_rect(x: u32, y: u32, width: u32, height: u32, radius: u32) -> bool {
    let r = radius as i32;
    let w = width as i32;
    let h = height as i32;
    let px = x as i32;
    let py = y as i32;

    // Check if in corner regions
    // Top-left corner
    if px < r && py < r {
        let dx = r - px;
        let dy = r - py;
        return dx * dx + dy * dy <= r * r;
    }
    // Top-right corner
    if px >= w - r && py < r {
        let dx = px - (w - r - 1);
        let dy = r - py;
        return dx * dx + dy * dy <= r * r;
    }
    // Bottom-left corner
    if px < r && py >= h - r {
        let dx = r - px;
        let dy = py - (h - r - 1);
        return dx * dx + dy * dy <= r * r;
    }
    // Bottom-right corner
    if px >= w - r && py >= h - r {
        let dx = px - (w - r - 1);
        let dy = py - (h - r - 1);
        return dx * dx + dy * dy <= r * r;
    }
    // Inside the main rectangle (not in corner cutout regions)
    true
}

// Render a combined image with icon + timer text for the menu bar
// This gives us full control over font size and positioning (unlike set_title)
fn render_tray_image(timer_text: Option<&str>) -> Vec<u8> {
    // Load the embedded font
    let font = Font::try_from_bytes(FONT_DATA).expect("Failed to load embedded font");

    // Load the icon and convert to black (the source is white for template mode)
    let mut icon_img = image::load_from_memory(ICON_INACTIVE)
        .expect("Failed to load tray icon")
        .to_rgba8();

    // Convert white pixels to black (keeping alpha)
    for pixel in icon_img.pixels_mut() {
        // Keep alpha, set RGB to black
        pixel[0] = 0; // R
        pixel[1] = 0; // G
        pixel[2] = 0; // B
        // pixel[3] stays as alpha
    }

    let icon_width = icon_img.width();
    let icon_height = icon_img.height();

    // For retina displays, we work at 2x resolution
    // Menu bar height is typically 22px, so our icon should be 22x22 (44x44 @2x)
    let canvas_height = 44u32; // @2x for retina

    // Padding and border radius settings (@2x for retina)
    let h_padding = 10u32; // 5px effective horizontal padding on each side
    let border_radius = 6u32; // 3px effective border radius - subtle rounding

    // Calculate text dimensions if we have timer text
    let (_text_width, content_width) = if let Some(text) = timer_text {
        // Font size: 36px @2x = 18pt effective - large for readability
        let scale = Scale::uniform(36.0);

        // Calculate text width using glyph metrics
        let glyphs: Vec<_> = font.layout(text, scale, rusttype::point(0.0, 0.0)).collect();
        let text_w = if let Some(last_glyph) = glyphs.last() {
            let pos = last_glyph.position();
            let h_metrics = last_glyph.unpositioned().h_metrics();
            (pos.x + h_metrics.advance_width) as u32 + 8 // Add padding
        } else {
            0
        };

        // Icon (scaled to 34x34 to leave room) + 14px gap + text
        let content_w = 34 + 14 + text_w;
        (text_w, content_w)
    } else {
        (0, 40) // Just the icon width (smaller when idle)
    };

    // Total canvas width includes padding on both sides
    let canvas_width = content_width + (h_padding * 2);

    // Create the canvas with transparent background first
    let mut canvas: RgbaImage = ImageBuffer::from_pixel(canvas_width, canvas_height, Rgba([0, 0, 0, 0]));

    // Draw the rounded rectangle glass background
    let glass_color = Rgba([255, 255, 255, 180]); // ~70% opacity white
    for y in 0..canvas_height {
        for x in 0..canvas_width {
            if is_inside_rounded_rect(x, y, canvas_width, canvas_height, border_radius) {
                canvas.put_pixel(x, y, glass_color);
            }
        }
    }

    // Scale icon to 34x34 @2x (17x17 effective) when showing text, 40x40 when idle
    let icon_size = if timer_text.is_some() { 34u32 } else { 40u32 };
    let scale_factor = icon_size as f32 / icon_height as f32;
    let scaled_icon = image::imageops::resize(
        &icon_img,
        (icon_width as f32 * scale_factor) as u32,
        icon_size,
        image::imageops::FilterType::Lanczos3
    );

    // Draw the icon centered vertically, with left padding
    let icon_x = h_padding;
    let icon_y = (canvas_height - scaled_icon.height()) / 2;
    image::imageops::overlay(&mut canvas, &scaled_icon, icon_x as i64, icon_y as i64);

    // Draw the timer text if present
    if let Some(text) = timer_text {
        let scale = Scale::uniform(36.0);

        // Get font metrics for vertical centering
        let v_metrics = font.v_metrics(scale);
        // Center the text vertically, then shift up 4px to visually balance
        let baseline_y = (canvas_height as f32 / 2.0) + (v_metrics.ascent / 2.0) - 4.0;

        // Text starts after padding + icon + gap
        let text_x = h_padding + 34 + 14;

        // Render in black - we won't use template mode to avoid flashing
        let text_color = Rgba([0, 0, 0, 255]); // Black for non-template mode

        for glyph in font.layout(text, scale, rusttype::point(text_x as f32, baseline_y)) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let px = bounding_box.min.x as u32 + x;
                    let py = bounding_box.min.y as u32 + y;
                    if px < canvas.width() && py < canvas.height() {
                        let alpha = (v * 255.0) as u8;
                        if alpha > 0 {
                            let pixel = canvas.get_pixel_mut(px, py);
                            // Blend with existing pixel
                            let existing_alpha = pixel[3] as f32 / 255.0;
                            let new_alpha = alpha as f32 / 255.0;
                            let combined_alpha = new_alpha + existing_alpha * (1.0 - new_alpha);
                            if combined_alpha > 0.0 {
                                pixel[0] = ((text_color[0] as f32 * new_alpha + pixel[0] as f32 * existing_alpha * (1.0 - new_alpha)) / combined_alpha) as u8;
                                pixel[1] = ((text_color[1] as f32 * new_alpha + pixel[1] as f32 * existing_alpha * (1.0 - new_alpha)) / combined_alpha) as u8;
                                pixel[2] = ((text_color[2] as f32 * new_alpha + pixel[2] as f32 * existing_alpha * (1.0 - new_alpha)) / combined_alpha) as u8;
                                pixel[3] = (combined_alpha * 255.0) as u8;
                            }
                        }
                    }
                });
            }
        }
    }

    // Encode to PNG
    let mut png_data: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    canvas.write_to(&mut cursor, image::ImageFormat::Png)
        .expect("Failed to encode PNG");

    png_data
}

// Update tray tooltip and icon based on timer state
#[tauri::command]
async fn update_tray_status(_app: tauri::AppHandle, is_running: bool, project_name: Option<String>) -> Result<(), String> {
    if let Some(tray) = TRAY_ICON.lock().unwrap().as_ref() {
        // Update tooltip
        let tooltip = if is_running {
            match project_name {
                Some(name) => format!("FreshTracks - {} (Running)", name),
                None => "FreshTracks - Timer Running".to_string(),
            }
        } else {
            "FreshTracks - No active timer".to_string()
        };
        tray.set_tooltip(Some(&tooltip)).map_err(|e| e.to_string())?;

        // Clear title when not running
        if !is_running {
            let _ = tray.set_title(None::<&str>);
        }

        // When not running, reset to the default rendered icon (black, no template)
        if !is_running {
            let icon_data = render_tray_image(None);
            if let Ok(icon) = Image::from_bytes(&icon_data) {
                let _ = tray.set_icon(Some(icon));
            }
        }
        // When running, the icon is handled by update_tray_timer
    }
    Ok(())
}

// Update tray icon with live timer display (rendered as combined image)
#[tauri::command]
async fn update_tray_timer(elapsed_seconds: i32) -> Result<(), String> {
    let hours = elapsed_seconds / 3600;
    let minutes = (elapsed_seconds % 3600) / 60;
    let seconds = elapsed_seconds % 60;

    // Format timer text (no leading space needed since we control positioning)
    let time_str = if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    };

    // Check if the text changed - skip update if same (prevents flickering)
    {
        let mut last_text = LAST_TIMER_TEXT.lock().unwrap();
        if last_text.as_ref() == Some(&time_str) {
            return Ok(()); // No change, skip update
        }
        *last_text = Some(time_str.clone());
    }

    if let Some(tray) = TRAY_ICON.lock().unwrap().as_ref() {
        // Render combined icon + text image
        let image_data = render_tray_image(Some(&time_str));

        // Set the rendered image as the tray icon (no template mode = no flash)
        if let Ok(icon) = Image::from_bytes(&image_data) {
            let _ = tray.set_icon(Some(icon));
        }
    }
    Ok(())
}

// Clear timer from tray (show only icon)
#[tauri::command]
async fn clear_tray_timer() -> Result<(), String> {
    // Clear the cache
    *LAST_TIMER_TEXT.lock().unwrap() = None;

    if let Some(tray) = TRAY_ICON.lock().unwrap().as_ref() {
        // Render just the icon without text
        let image_data = render_tray_image(None);

        // No template mode = no flash
        if let Ok(icon) = Image::from_bytes(&image_data) {
            let _ = tray.set_icon(Some(icon));
        }
    }
    Ok(())
}

// Sync timer state from frontend to backend (for shortcut toggle)
#[tauri::command]
async fn sync_timer_state(timer: Option<TimeEntry>, first_project_id: Option<i32>) -> Result<(), String> {
    *ACTIVE_TIMER_STATE.lock().unwrap() = timer;
    if let Some(pid) = first_project_id {
        *FIRST_PROJECT_ID.lock().unwrap() = Some(pid);
    }
    Ok(())
}

// Show macOS notification
#[tauri::command]
async fn show_notification(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Toggle timer via global shortcut
async fn toggle_timer_via_shortcut(app: &tauri::AppHandle) {
    // Get current timer state
    let timer_state = ACTIVE_TIMER_STATE.lock().unwrap().clone();

    if let Some(timer) = timer_state {
        // Timer is running - stop it
        match stop_timer_internal(timer.id).await {
            Ok(stopped) => {
                // Update state
                *ACTIVE_TIMER_STATE.lock().unwrap() = None;

                // Calculate duration for notification
                let duration_str = match stopped.duration_minutes {
                    Some(mins) => {
                        let total_minutes = mins as i32;
                        let hours = total_minutes / 60;
                        let minutes = total_minutes % 60;
                        if hours > 0 {
                            format!("{}h {}m", hours, minutes)
                        } else {
                            format!("{}m", minutes)
                        }
                    }
                    None => "".to_string()
                };

                // Show notification
                let project_name = stopped.project.as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Timer".to_string());

                let _ = app.notification()
                    .builder()
                    .title("Timer Stopped")
                    .body(&format!("{} - {}", project_name, duration_str))
                    .show();

                // Update tray tooltip and icon (rendered black, no template)
                if let Some(tray) = TRAY_ICON.lock().unwrap().as_ref() {
                    let _ = tray.set_tooltip(Some("FreshTracks - No active timer"));
                    let icon_data = render_tray_image(None);
                    if let Ok(icon) = Image::from_bytes(&icon_data) {
                        let _ = tray.set_icon(Some(icon));
                    }
                }

                // Emit event to frontend to sync UI
                let _ = app.emit("timer-toggled", ());
            }
            Err(e) => {
                eprintln!("Failed to stop timer via shortcut: {}", e);
            }
        }
    } else {
        // No timer running - start one with first project
        let first_project = FIRST_PROJECT_ID.lock().unwrap().clone();

        if let Some(project_id) = first_project {
            match start_timer_internal(project_id, None).await {
                Ok(started) => {
                    // Update state
                    *ACTIVE_TIMER_STATE.lock().unwrap() = Some(started.clone());

                    // Show notification
                    let project_name = started.project.as_ref()
                        .map(|p| {
                            if let Some(client) = &p.client {
                                format!("{} - {}", client.name, p.name)
                            } else {
                                p.name.clone()
                            }
                        })
                        .unwrap_or_else(|| "Project".to_string());

                    let _ = app.notification()
                        .builder()
                        .title("Timer Started")
                        .body(&project_name)
                        .show();

                    // Update tray tooltip (icon will be handled by frontend calling update_tray_timer)
                    if let Some(tray) = TRAY_ICON.lock().unwrap().as_ref() {
                        let _ = tray.set_tooltip(Some(&format!("FreshTracks - {} (Running)", project_name)));
                    }

                    // Emit event to frontend to sync UI
                    let _ = app.emit("timer-toggled", ());
                }
                Err(e) => {
                    eprintln!("Failed to start timer via shortcut: {}", e);
                }
            }
        } else {
            // No project available - show window instead
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

// Internal helper for starting timer (without tauri::command wrapper)
async fn start_timer_internal(project_id: i32, description: Option<String>) -> Result<TimeEntry, String> {
    let client = get_http_client();

    #[derive(Serialize)]
    struct StartTimerRequest {
        project_id: i32,
        description: Option<String>,
    }

    let body = StartTimerRequest {
        project_id,
        description,
    };

    let response = client
        .post(&format!("{}/time-entries", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let timer = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(timer)
}

// Internal helper for stopping timer
async fn stop_timer_internal(timer_id: i32) -> Result<TimeEntry, String> {
    let client = get_http_client();

    let response = client
        .post(&format!("{}/time-entries/{}/stop", API_BASE, timer_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let timer = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(timer)
}

// API Commands
#[tauri::command]
async fn get_active_timer() -> Result<Option<TimeEntry>, String> {
    let client = get_http_client();
    let response = client
        .get(&format!("{}/active-timer", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 404 || response.status() == 204 {
        return Ok(None);
    }

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    // Laravel returns null as JSON when no active timer
    let text = response.text().await.map_err(|e| e.to_string())?;
    if text == "null" || text.is_empty() {
        return Ok(None);
    }

    let timer: TimeEntry = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(Some(timer))
}

#[tauri::command]
async fn start_timer(project_id: i32, description: Option<String>) -> Result<TimeEntry, String> {
    let client = get_http_client();

    #[derive(Serialize)]
    struct StartTimerRequest {
        project_id: i32,
        description: Option<String>,
    }

    let body = StartTimerRequest {
        project_id,
        description,
    };

    let response = client
        .post(&format!("{}/time-entries", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let timer = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(timer)
}

#[tauri::command]
async fn stop_timer(timer_id: i32) -> Result<TimeEntry, String> {
    let client = get_http_client();

    let response = client
        .post(&format!("{}/time-entries/{}/stop", API_BASE, timer_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let timer = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(timer)
}

#[tauri::command]
async fn get_time_entries() -> Result<Vec<TimeEntry>, String> {
    let client = get_http_client();

    let response = client
        .get(&format!("{}/time-entries", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let entries: Vec<TimeEntry> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(entries)
}

#[tauri::command]
async fn restart_timer(timer_id: i32) -> Result<TimeEntry, String> {
    let client = get_http_client();

    let response = client
        .post(&format!("{}/time-entries/{}/restart", API_BASE, timer_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let timer = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(timer)
}

#[tauri::command]
async fn delete_entry(entry_id: i32) -> Result<(), String> {
    let client = get_http_client();

    let response = client
        .delete(&format!("{}/time-entries/{}", API_BASE, entry_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    Ok(())
}

#[tauri::command]
async fn update_entry(entry_id: i32, data: UpdateEntryData) -> Result<TimeEntry, String> {
    let client = get_http_client();

    let response = client
        .put(&format!("{}/time-entries/{}", API_BASE, entry_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .json(&data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }

    let entry = response
        .json::<TimeEntry>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(entry)
}

#[tauri::command]
async fn get_recent_projects() -> Result<Vec<Project>, String> {
    let client = get_http_client();

    let response = client
        .get(&format!("{}/projects", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let projects: Vec<Project> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
async fn get_clients() -> Result<Vec<Client>, String> {
    let client = get_http_client();

    let response = client
        .get(&format!("{}/clients", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let clients: Vec<Client> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(clients)
}

#[tauri::command]
async fn get_client_projects(client_id: i32) -> Result<Vec<Project>, String> {
    let client = get_http_client();

    let response = client
        .get(&format!("{}/clients/{}/projects", API_BASE, client_id))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let projects: Vec<Project> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
async fn get_settings() -> Result<UserSettings, String> {
    let client = get_http_client();

    let response = client
        .get(&format!("{}/settings", API_BASE))
        .header("X-Widget-Secret", WIDGET_SECRET)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("Widget authentication failed. Check your secret.".to_string());
    }

    if !response.status().is_success() {
        return Ok(UserSettings::default());
    }

    let settings: UserSettings = response
        .json()
        .await
        .unwrap_or_default();

    Ok(settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin({
            // Global shortcut plugin with handler
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            toggle_timer_via_shortcut(&app_clone).await;
                        });
                    }
                })
                .build()
        })
        .setup(|app| {
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Register global shortcut (Cmd+Shift+T)
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyT);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("Failed to register global shortcut: {}", e);
            } else {
                println!("Global shortcut Cmd+Shift+T registered successfully");
            }

            // Start WebSocket connection for real-time sync
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                connect_websocket(app_handle).await;
            });

            // Create tray menu
            let quit_item = MenuItem::with_id(app, "quit", "Quit FreshTracks", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show Timer", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Load tray icon using our render function (black, non-template)
            let icon_data = render_tray_image(None);
            let icon = Image::from_bytes(&icon_data)
                .expect("Failed to load tray icon");

            // Build tray icon (no template mode to avoid flashing during updates)
            let tray = TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(false)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("FreshTracks - Time Tracking")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                // Position window below tray icon
                                let window_size = window.outer_size().unwrap_or_default();
                                let x = position.x as i32 - (window_size.width as i32 / 2);
                                let y = position.y as i32 + 5;

                                let _ = window.set_position(PhysicalPosition::new(x, y));
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Store tray reference for later updates
            *TRAY_ICON.lock().unwrap() = Some(tray);

            // Hide window when it loses focus
            let main_window = app.get_webview_window("main").unwrap();
            let window_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window_clone.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_active_timer,
            start_timer,
            stop_timer,
            restart_timer,
            delete_entry,
            update_entry,
            get_time_entries,
            get_recent_projects,
            get_clients,
            get_client_projects,
            get_settings,
            update_tray_status,
            update_tray_timer,
            clear_tray_timer,
            sync_timer_state,
            show_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
