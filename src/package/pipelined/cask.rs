use super::{
    super::{
        Packageable,
        prepared::{PreparedCask, Stanzas},
    },
    PipelinedPackageable,
};

pub(crate) struct PipelinedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    variation_stanzas: Stanzas,
    pub(in super::super) is_requested: bool,
}

impl From<PreparedCask> for PipelinedCask {
    fn from(prepared_cask: PreparedCask) -> Self {
        Self {
            token: prepared_cask.token,
            version: prepared_cask.version,
            variation_stanzas: prepared_cask.variation_stanzas,
            is_requested: prepared_cask.is_requested,
        }
    }
}

impl Packageable for PipelinedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl PipelinedPackageable for PipelinedCask {}

impl PipelinedCask {
    pub(crate) fn stanzas(&self) -> &Stanzas {
        &self.variation_stanzas
    }
}
