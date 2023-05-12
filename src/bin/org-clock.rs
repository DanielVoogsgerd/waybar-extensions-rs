use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use notify_rust::Notification;
use std::cell::RefCell;
use waybar_extensions::config::{Config, OrgClockConfig};

type BoxedError = Box<dyn std::error::Error>;

const CLOCK_MARKER: &str = "org-clock-marker";
const CLOCK_CURRENT_TASK: &str = "org-clock-heading";
const CLOCK_IN_TIME: &str = "(time-to-seconds org-clock-start-time)";

struct State {
    state: Option<ClockProperties>,
}

#[derive(Clone)]
struct ClockProperties {
    task: String,
    time: DateTime<Local>,
}

#[tokio::main]
async fn main() {
    let state = RefCell::new(State { state: None });

    let updater = update_loop(&state);
    let notifier = notify_loop(&state);
    let printer = print_loop(&state);

    let futures = futures::future::join3(updater, notifier, printer);

    futures.await;
}

async fn run_emacs_command(emacs_command: &str) -> Result<String, Box<dyn std::error::Error>> {
    let command = tokio::process::Command::new("emacsclient")
        .arg("--eval")
        .arg(emacs_command)
        .output()
        .await?;
    Ok(String::from_utf8(command.stdout)?.trim_end().to_string())
}

async fn clock_running() -> Result<bool, Box<dyn std::error::Error>> {
    let command = run_emacs_command(CLOCK_MARKER).await?;
    Ok(command != "#<marker in no buffer>")
}

async fn get_task() -> Result<String, Box<dyn std::error::Error>> {
    let command = run_emacs_command(CLOCK_CURRENT_TASK).await?;
    Ok(command.trim_matches('"').to_string())
}

async fn get_start_time() -> Result<DateTime<Local>, Box<dyn std::error::Error>> {
    let command = run_emacs_command(CLOCK_IN_TIME).await?;
    let start_time_float = command.parse::<f64>()?;
    let naive_time = NaiveDateTime::from_timestamp_opt(
        start_time_float as i64,
        (start_time_float % 1f64 * 1e9) as u32,
    )
    .ok_or("Could not convert timestamp to datetime")?;
    let start_time_utc = DateTime::<Utc>::from_utc(naive_time, Utc);
    let start_time = DateTime::<Local>::from(start_time_utc);
    Ok(start_time)
}

async fn updater(state: &RefCell<State>) -> Result<(), BoxedError> {
    if clock_running().await? {
        let task = get_task().await?;
        let time = get_start_time().await?;
        state.borrow_mut().state = Some(ClockProperties { task, time })
    } else {
        state.borrow_mut().state = None;
    }

    Ok(())
}

async fn update_loop(state: &RefCell<State>) {
    loop {
        if updater(state).await.is_err() {
            eprintln!("Something went wrong when checking clock");
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn print_loop(state: &RefCell<State>) {
    loop {
        let now = Local::now();

        let text = if let Some(clock_properties) = &state.borrow().state {
            let duration = now - clock_properties.time;
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() % 60;
            let seconds = duration.num_seconds() % 60;
            format!(
                "{}: {hours:02}:{minutes:02}:{seconds:02}",
                clock_properties.task
            )
        } else {
            "Untracked time".to_string()
        };

        let response = waybar_extensions::waybar::WaybarResponse {
            text,
            tooltip: "".to_string(),
            class: vec![],
        };

        if let Ok(result) = serde_json::to_string(&response) {
            println!("{result}");
        } else {
            eprintln!("Could not format waybar response");
        }

        let now = Local::now();
        let wait_duration = 1_000_000_000u32 - now.timestamp_subsec_nanos();
        tokio::time::sleep(std::time::Duration::from_nanos(wait_duration.into())).await;
    }
}

async fn notify_loop(state: &RefCell<State>) {
    match Config::load("waybar", "modules.toml") {
        Ok(config) => loop {
            let now = Local::now();
            let delay = {
                let state_ref = state.borrow();

                if let Some(clock_properties) = &state_ref.state {
                    let delta = now - clock_properties.time;
                    if delta.num_minutes() as u32 > config.org_clock.notify_time
                        && Notification::new()
                            .summary("Time for a break")
                            .body(&format!(
                                "You've worked for {} minutes",
                                delta.clone().num_minutes()
                            ))
                            .show()
                            .is_err()
                    {
                        eprintln!("Could not send notification");
                    }
                    get_notify_sleep_time(&config.org_clock, &delta)
                } else {
                    std::time::Duration::from_secs((60 * config.org_clock.notify_interval).into())
                }
            };
            tokio::time::sleep(delay).await;
        },
        Err(_e) => {
            eprintln!("Could not load configuration, will not be showing notifications");
        }
    }
}

fn get_notify_sleep_time(clock_config: &OrgClockConfig, delta: &Duration) -> std::time::Duration {
    let delta_secs: u64 = delta
        .num_seconds()
        .try_into()
        .expect("Could not cast duration to u32");
    let notify_time: u64 = clock_config.notify_time as u64 * 60;
    let notify_interval: u64 = clock_config.notify_interval as u64 * 60;

    let offset = notify_time % notify_interval;
    std::time::Duration::from_secs(
        ((delta_secs + notify_interval - offset) / notify_interval) * notify_interval + offset
            - delta_secs,
    )
}
