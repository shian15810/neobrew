use super::{
    super::{
        Packageable,
        prepared::{PreparedCask, Stanzas},
    },
    StreamedPackageable,
};

pub(crate) struct StreamedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    variation_stanzas: Stanzas,
    pub(in super::super) is_requested: bool,
}

impl From<PreparedCask> for StreamedCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            variation_stanzas: prepared_cask.variation_stanzas,
            is_requested: prepared_cask.is_requested,
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

impl StreamedPackageable for StreamedCask {}

impl StreamedCask {
    pub(crate) fn stanzas(&self) -> &Stanzas {
        &self.variation_stanzas
    }
}
