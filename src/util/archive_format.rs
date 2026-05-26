use std::path::Path;

use crate::ext::std::path::PathExt as _;

pub(crate) enum ArchiveFormat {
    TarGz,
    Zip,
}

impl TryFrom<&Path> for ArchiveFormat {
    type Error = Option<anyhow::Error>;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let Some(compound_extension) = path.compound_extension() else {
            return Err(None);
        };

        let temp_pourer_format = match compound_extension.to_string_lossy().as_ref() {
            "tar.gz" | "tgz" => Self::TarGz,
            "zip" => Self::Zip,
            _ => return Err(None),
        };

        Ok(temp_pourer_format)
    }
}
