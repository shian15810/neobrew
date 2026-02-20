use serde::Deserialize;

use super::Packageable;

#[derive(Deserialize)]
pub struct Cask {
    token: String,
    name: Vec<String>,
    url: String,
    version: String,
    sha256: String,
}

impl Packageable for Cask {
    fn id(&self) -> &str {
        &self.token
    }
}
