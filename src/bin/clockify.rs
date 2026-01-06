use anyhow::Result;
use chrono::{DateTime, Local, Timelike};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Mutex, Notify};
use waybar_extensions::waybar::WaybarResponse;

const SOCKET_NAME: &str = "waybar-clockify.sock";
const UPDATE_INTERVAL: Duration = Duration::from_secs(300);
const SOCKET_REFRESH_COMMAND: &str = "refresh";
const UNTRACKED_MESSAGE: &str = "Untracked time";
const UNKNOWN_TASK: &str = "Unknown task";

#[derive(Clone)]
struct ClockifyStatus {
    task: String,
    start_time: DateTime<Local>,
}

fn get_socket_path() -> PathBuf {
    xdg::BaseDirectories::new()
        .ok()
        .and_then(|xdg| xdg.get_runtime_directory().ok().cloned())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(SOCKET_NAME)
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(None::<ClockifyStatus>));
    let update_trigger = Arc::new(Notify::new());

    let updater = update_loop(Arc::clone(&state), Arc::clone(&update_trigger));
    let printer = print_loop(Arc::clone(&state));
    let socket_handler = socket_loop(Arc::clone(&update_trigger));

    tokio::join!(updater, printer, socket_handler);
}

async fn get_clockify_status() -> Result<Option<ClockifyStatus>> {
    let output = tokio::process::Command::new("clockify-cli")
        .arg("show")
        .arg("current")
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    if line.is_empty() || line == "null" || line == "[]" {
        return Ok(None);
    }

    let json: serde_json::Value = serde_json::from_str(line)?;

    // Handle array response - take first element
    let entry = if json.is_array() {
        json.get(0)
            .ok_or_else(|| anyhow::anyhow!("Empty array returned"))?
    } else {
        &json
    };

    let task = entry["description"]
        .as_str()
        .or_else(|| entry["task"]["name"].as_str())
        .or_else(|| entry["project"]["name"].as_str())
        .unwrap_or(UNKNOWN_TASK)
        .to_string();

    let start_time_str = entry["timeInterval"]["start"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No start time found"))?;

    let start_time =
        DateTime::parse_from_rfc3339(start_time_str).map(|dt| dt.with_timezone(&Local))?;

    Ok(Some(ClockifyStatus { task, start_time }))
}

async fn update_loop(state: Arc<Mutex<Option<ClockifyStatus>>>, trigger: Arc<Notify>) {
    loop {
        match get_clockify_status().await {
            Ok(status) => {
                *state.lock().await = status;
            }
            Err(e) => {
                eprintln!("Error getting clockify status: {}", e);
                *state.lock().await = None;
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(UPDATE_INTERVAL) => {},
            _ = trigger.notified() => {},
        }
    }
}

async fn socket_loop(trigger: Arc<Notify>) {
    let socket_path = get_socket_path();

    // Remove old socket if it exists
    let _ = std::fs::remove_file(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind socket at {:?}: {}", socket_path, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                handle_socket_connection(stream, Arc::clone(&trigger)).await;
            }
            Err(e) => {
                eprintln!("Socket accept error: {}", e);
            }
        }
    }
}

async fn handle_socket_connection(stream: UnixStream, trigger: Arc<Notify>) {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim() == SOCKET_REFRESH_COMMAND {
            trigger.notify_one();
        }
    }
}

fn format_duration(status: &ClockifyStatus) -> String {
    let duration = Local::now() - status.start_time;
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;
    format!(
        "{}: {:02}:{:02}:{:02}",
        status.task, hours, minutes, seconds
    )
}

async fn print_loop(state: Arc<Mutex<Option<ClockifyStatus>>>) {
    loop {
        let text = {
            let guard = state.lock().await;
            guard
                .as_ref()
                .map(format_duration)
                .unwrap_or_else(|| UNTRACKED_MESSAGE.to_string())
        };

        let response = WaybarResponse {
            text,
            tooltip: String::new(),
            class: vec![],
        };

        if let Ok(result) = serde_json::to_string(&response) {
            println!("{result}");
        } else {
            eprintln!("Could not format waybar response");
        }

        let now = Local::now();
        let next_second = now + chrono::Duration::seconds(1);
        let next_second = next_second.with_nanosecond(0).unwrap_or(next_second);

        let sleep_duration = (next_second - now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(1));

        tokio::time::sleep(sleep_duration).await;
    }
}
