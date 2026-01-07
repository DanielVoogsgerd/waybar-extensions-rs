use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use clap::Parser;
use serde::Deserialize;
use std::process::Stdio;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use waybar_extensions::waybar::WaybarResponse;

const UPDATE_INTERVAL: Duration = Duration::from_secs(300);
const STARTING_THRESHOLD: Duration = Duration::from_secs(15 * 60);

#[derive(Parser, Debug)]
#[command(name = "khal")]
#[command(about = "Waybar khal calendar widget")]
struct Args {
    /// Run once and exit (for testing)
    #[arg(long)]
    single: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct KhalEvent {
    #[serde(alias = "summary")]
    title: String,
    start: String,
    end: String,
    #[serde(default)]
    location: String,
}

impl KhalEvent {
    fn start_datetime(&self) -> Result<DateTime<Local>> {
        DateTime::parse_from_rfc3339(&self.start)
            .map(|dt| dt.with_timezone(&Local))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&self.start, "%Y-%m-%d %H:%M")
                    .ok()
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .or_else(|_| {
                NaiveDate::parse_from_str(&self.start, "%Y-%m-%d")
                    .ok()
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&self.start, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .map_err(|_| anyhow::anyhow!("Could not parse start time: {}", self.start))
    }

    fn end_datetime(&self) -> Result<DateTime<Local>> {
        DateTime::parse_from_rfc3339(&self.end)
            .map(|dt| dt.with_timezone(&Local))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&self.end, "%Y-%m-%d %H:%M")
                    .ok()
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .or_else(|_| {
                NaiveDate::parse_from_str(&self.end, "%Y-%m-%d")
                    .ok()
                    .and_then(|date| date.and_hms_opt(23, 59, 59))
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&self.end, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .and_then(|dt| Local.from_local_datetime(&dt).single())
                    .ok_or_else(|| anyhow::anyhow!("Failed to parse"))
            })
            .map_err(|_| anyhow::anyhow!("Could not parse end time: {}", self.end))
    }

    fn is_ongoing(&self, now: DateTime<Local>) -> bool {
        matches!(
            (self.start_datetime(), self.end_datetime()),
            (Ok(start), Ok(end)) if now >= start && now < end
        )
    }

    fn is_starting_soon(&self, now: DateTime<Local>) -> bool {
        self.start_datetime()
            .ok()
            .and_then(|start| (start - now).to_std().ok())
            .map(|duration_until| duration_until > Duration::ZERO && duration_until <= STARTING_THRESHOLD)
            .unwrap_or(false)
    }

    fn format_time(&self) -> String {
        self.start_datetime()
            .ok()
            .map(|start| start.format("%H:%M").to_string())
            .unwrap_or_else(|| "??:??".to_string())
    }

    fn format_tooltip(&self) -> String {
        if self.location.is_empty() {
            format!("{} - {}", self.format_time(), self.title)
        } else {
            format!(
                "{} - {} ({})",
                self.format_time(),
                self.title,
                self.location
            )
        }
    }
}

async fn get_today_events() -> Result<Vec<KhalEvent>> {
    let output = tokio::process::Command::new("khal")
        .args(["list", "--json", "title", "--json", "start", "--json", "end", "--json", "location", "today"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("khal command failed: {}", stderr);
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    debug!("khal raw output: {}", stdout);

    let events: Vec<KhalEvent> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .flat_map(|line| {
            debug!("Parsing line: {}", line);
            match serde_json::from_str::<Vec<KhalEvent>>(line) {
                Ok(line_events) => {
                    debug!("Parsed {} events from line", line_events.len());
                    for event in &line_events {
                        debug!(
                            "Event: title={:?}, start={:?}, end={:?}, location={:?}",
                            event.title, event.start, event.end, event.location
                        );
                    }
                    line_events
                }
                Err(e) => {
                    warn!("Failed to parse event line '{}': {}", line, e);
                    Vec::new()
                }
            }
        })
        .collect();

    info!("Total valid events found: {}", events.len());
    Ok(events)
}

fn get_next_event(events: &[KhalEvent], now: DateTime<Local>) -> Option<&KhalEvent> {
    events
        .iter()
        .filter(|event| {
            event
                .start_datetime()
                .ok()
                .map(|start| start > now)
                .unwrap_or(false)
        })
        .min_by_key(|event| event.start_datetime().ok())
}

fn format_output(events: &[KhalEvent]) -> WaybarResponse {
    let now = Local::now();

    events
        .iter()
        .find(|e| e.is_ongoing(now))
        .map(|ongoing| {
            info!("Ongoing event: {}", ongoing.title);
            WaybarResponse {
                text: format!(" {}", ongoing.title),
                tooltip: events.iter().map(|e| e.format_tooltip()).collect::<Vec<_>>().join("\n"),
                class: vec!["started".to_string()],
            }
        })
        .or_else(|| {
            get_next_event(events, now).map(|next| {
                info!("Next event: {}", next.title);
                let class = if next.is_starting_soon(now) {
                    info!("Next event starting soon");
                    vec!["starting".to_string()]
                } else {
                    vec![]
                };
                WaybarResponse {
                    text: format!("{} {}", next.format_time(), next.title),
                    tooltip: events.iter().map(|e| e.format_tooltip()).collect::<Vec<_>>().join("\n"),
                    class,
                }
            })
        })
        .unwrap_or_else(|| {
            debug!("No events found");
            WaybarResponse {
                text: "No events".to_string(),
                tooltip: String::new(),
                class: vec![],
            }
        })
}

async fn print_once() {
    let events = match get_today_events().await {
        Ok(events) => events,
        Err(e) => {
            error!("Error getting khal events: {}", e);
            let response = WaybarResponse {
                text: "Error".to_string(),
                tooltip: format!("Error: {}", e),
                class: vec![],
            };
            println!("{}", serde_json::to_string(&response).unwrap_or_default());
            return;
        }
    };

    let response = format_output(&events);
    println!(
        "{}",
        serde_json::to_string(&response).unwrap_or_else(|e| {
            error!("Could not format waybar response: {}", e);
            String::new()
        })
    );
}

async fn update_loop() {
    loop {
        print_once().await;
        tokio::time::sleep(UPDATE_INTERVAL).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    if args.single {
        info!("Running in single mode");
        print_once().await;
    } else {
        info!("Starting continuous update loop");
        update_loop().await;
    }

    Ok(())
}
