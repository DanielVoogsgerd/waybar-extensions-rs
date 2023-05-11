use std::path::PathBuf;
use std::process::Command;

fn get_current_kernel() -> Result<String, Box<dyn std::error::Error>> {
    let command_output = Command::new("uname").arg("-r").output()?;
    let mut output_string = String::from_utf8(command_output.stdout)?;

    // Remove newline
    output_string.pop();

    Ok(output_string)
}

fn loaded_kernel_has_modules_installed() -> Result<bool, Box<dyn std::error::Error>> {
    let loaded_kernel = get_current_kernel();

    let mut modules_path = PathBuf::from("/lib/modules");
    modules_path.push(loaded_kernel?);

    Ok(modules_path.exists())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outdated_kernel = !loaded_kernel_has_modules_installed()?;

    let command_output = Command::new("systemctl")
        .arg("--user")
        .arg("list-units")
        .arg("--failed")
        .output()?;

    let output_string = String::from_utf8(command_output.stdout)?;

    let failed_units = output_string
        .split('\n')
        .skip(1)
        .take_while(|&row| !row.is_empty())
        .filter_map(|row| {
            if row.split_ascii_whitespace().nth(3)? == "failed" {
                return row.split_ascii_whitespace().nth(1);
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let warning_count = failed_units.len() + if outdated_kernel { 1 } else { 0 };

    let class = if !failed_units.is_empty() {
        vec!["critical".to_owned()]
    } else if outdated_kernel {
        vec!["warning".to_owned()]
    } else {
        vec![]
    };

    let mut tooltip: String = failed_units.join("\n");

    if outdated_kernel {
        tooltip.push_str("\nLoaded kernel is outdated");
    }

    let waybar_response = waybar_extensions::waybar::WaybarResponse {
        text: warning_count.to_string(),
        tooltip,
        class,
    };

    println!(
        "{}",
        serde_json::to_string(&waybar_response).expect("Cannot format a waybar response")
    );

    Ok(())
}
