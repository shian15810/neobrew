use super::super::Packageable;

pub(crate) struct InstalledFormula {
    name: String,
    version_revision: String,
}

impl Packageable for InstalledFormula {
    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version_revision
    }
}
