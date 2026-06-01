use super::super::{
    Packageable,
    prepared::{PreparedCask, Stanzas},
};

pub(crate) struct StreamedCask {
    token: String,
    version: String,
    pub(crate) variation_stanzas: Stanzas,
}

impl From<PreparedCask> for StreamedCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            variation_stanzas: prepared_cask.variation_stanzas,
        }
    }
}

impl Packageable for StreamedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}
