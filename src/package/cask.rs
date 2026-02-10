use serde::Deserialize;

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}
