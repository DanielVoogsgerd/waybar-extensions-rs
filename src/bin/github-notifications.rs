use std::time::Duration;

use itertools::Itertools;
use serde_json::json;
use tokio::process::Command;
use waybar_extensions::waybar::WaybarResponse;

#[tokio::main]
async fn main() {
    use octocrab::Octocrab;

    // let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env variable is required");
    let token = String::from_utf8(
        Command::new("pass")
            .arg("Background/GitHub")
            .output()
            .await
            .expect("Could not get Github token")
            .stdout,
    )
    .expect("Github token is not valid UTF-8")
    .trim_end()
    .to_owned();

    let octocrab = Octocrab::builder()
        .personal_token(token)
        .build()
        .expect("Could not create octocrab instance");

    loop {
        if let Ok(notifications) = octocrab.activity().notifications().list().send().await {
            let text = format!(" {}", notifications.items.len());

            let tooltip = notifications
                .items
                .iter()
                .map(|notification| {
                    format!(
                        "{}  {}",
                        get_icon(&notification.subject.r#type),
                        notification.subject.title
                    )
                })
                .join("\n");

            println!(
                "{}",
                json!(WaybarResponse {
                    text,
                    tooltip,
                    class: vec![],
                })
            );
        } else {
            println!(
                "{}",
                json!(WaybarResponse {
                    text: String::from(" "),
                    tooltip: String::from("Could not request notifications"),
                    class: vec![],
                })
            );
        }

        tokio::time::sleep(Duration::from_secs(300)).await;
    }
}

fn get_icon(x: &str) -> &str {
    match x {
        "PullRequest" => "",
        "Issue" => "",
        "Discussion" => "",
        "Release" => "",
        y => y,
    }
}
