use super::super::Packageable;

pub(crate) struct InstalledCask {
    token: String,
    version: String,
}

impl Packageable for InstalledCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}
