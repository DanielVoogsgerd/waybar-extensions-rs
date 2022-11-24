use std::{
    error::Error,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use git2::{Repository, Status};
use waybar_extensions::config::Config;

#[derive(serde::Serialize)]
pub struct WaybarResponse {
    text: String,
    tooltip: String,
    class: Vec<String>,
}

/// Starting project dir
const PROJECT_DIR: &str = "/home/daniel/dev";

/// Statuses to ignore
const IGNORE_STATUSES: Status = Status::IGNORED.union(Status::WT_DELETED);

fn main() {
    let start_dir = Path::new(PROJECT_DIR);

    let config = Config::load("waybar", "modules.toml").expect("Could not load configuration");

    let repos = get_dirs(start_dir)
        .expect("Could not read dir")
        .filter(|dir| is_git_repo(dir))
        .filter_map(|project_path| {
            let repo = Repository::open(&project_path).expect("Could not open repository");
            let status = repo
                .statuses(None)
                .expect("Could not get status")
                .iter()
                .filter_map(|x| Some((x.path()?.to_owned(), x.status())))
                .collect::<Vec<_>>();

            let uncommitted_files = status
                .iter()
                .filter_map(|(path, status)| {
                    (!IGNORE_STATUSES.intersects(*status))
                        .then_some((project_path.join(path), *status))
                })
                .map(|(path, status)| {
                    (
                        path.clone(),
                        path_age(&path, config.unfinished_projects.max_file_depth),
                        status,
                    )
                })
                .collect::<Vec<_>>();

            (uncommitted_files.len() != 0).then_some((project_path, uncommitted_files))
        })
        .collect::<Vec<_>>();

    let output = repos
        .iter()
        .filter(|(_project_path, uncommitted_files)| {
            if let Some((_, Some(max_age), _)) = uncommitted_files
                .iter()
                .max_by_key(|(_path, age, _status)| age)
            {
                *max_age > Duration::new(60 * 60 * 24 * config.unfinished_projects.max_age, 0)
            } else {
                false
            }
        })
        .collect::<Vec<_>>();

    let mut tooltip = String::new();
    for (project_path, uncommitted_files) in output.iter() {
        tooltip.push_str(&format!(
            "{:} has {} uncommitted changes\n",
            project_path
                .file_name()
                .expect("Could not get filename")
                .to_string_lossy(),
            uncommitted_files.len()
        ))
    }

    let text = format!("{}", output.len());

    let response = WaybarResponse {
        text,
        tooltip,
        class: vec![],
    };
    let waybar_response =
        serde_json::to_string(&response).expect("Could not format waybar response");

    println!("{:}", waybar_response);
}

/// Path age takes a path and checks how long ago it was last modified.
/// If it is a directory it will recursively search the directory and return the oldest entry's age.
fn path_age(path: &Path, max_depth: Option<usize>) -> Option<Duration> {
    if max_depth == Some(0) {
        return None;
    }

    if path.is_dir() {
        path.read_dir()
            .ok()
            .unwrap_or_else(|| panic!("Could not read directory {path:?}"))
            .filter_map(|x| x.ok())
            .filter_map(|entry| path_age(&entry.path(), max_depth.map(|x| x - 1)))
            .max()
    } else {
        Some(
            SystemTime::now()
                .duration_since(path.metadata().ok()?.modified().ok()?)
                .ok()?,
        )
    }
}

fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

fn get_dirs(path: &Path) -> Result<impl Iterator<Item = PathBuf> + '_, Box<dyn Error>> {
    Ok(path
        .read_dir()?
        .filter_map(|x| x.ok())
        .filter_map(|x| x.file_type().ok()?.is_dir().then_some(x.path())))
}
