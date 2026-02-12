use serde::Deserialize;

#[derive(Deserialize)]
pub struct Cask {
    pub token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}
