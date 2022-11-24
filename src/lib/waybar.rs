use serde::Serialize;

#[derive(Serialize)]
pub struct WaybarResponse {
    pub text: String,
    pub tooltip: String,
    pub class: Vec<String>,
}
