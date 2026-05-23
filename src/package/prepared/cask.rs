use std::path::Path;

use anyhow::{Context as _, Result};
use base16ct::HexDisplay;
use sha2::{Digest as _, Sha256};
use url::Url;

use super::{
    super::{Packageable, resolved::ResolvedCask},
    PreparedPackageable,
};
use crate::{
    context::{Context, dirs::ProjectDirs as _},
    pipeline::push_operator::TempWriterInput,
};

pub(crate) struct PreparedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    url: String,
    sha256: String,
}

impl From<ResolvedCask> for PreparedCask {
    fn from(resolved_cask: ResolvedCask) -> Self {
        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version = {
            use super::super::resolved::ResolvedPackageable as _;

            resolved_cask.version()
        };

        #[cfg(not(debug_assertions))]
        let version = {
            use super::super::resolved::ResolvedPackageable;

            ResolvedPackageable::version(&resolved_cask)
        };

        let version = version.into_owned();

        Self {
            token: resolved_cask.token,
            version,
            url: resolved_cask.url,
            sha256: resolved_cask.sha256,
        }
    }
}

impl Packageable for PreparedCask {
    fn id(&self) -> &str {
        &self.token
    }

    fn version(&self) -> &str {
        &self.version
    }
}

impl PreparedPackageable for PreparedCask {
    fn expected_sha256(&self) -> &str {
        &self.sha256
    }

    async fn temp_writer_input(&self, context: &Context) -> Result<TempWriterInput> {
        let version = self.version();

        let url = Url::parse(&self.url)?;

        let mut segment = url.path_segments().context("Invalid URL")?;
        let segment = segment.next_back().context("Empty URL path segments")?;

        let path = Path::new(segment);

        let extension = path.extension().context("Invalid file name")?;
        let extension = extension.to_str().context("Invalid file extension")?;

        let url_hash = Sha256::digest(&self.url);
        let url_hash = HexDisplay(&url_hash);
        let url_hash = format!("{url_hash:x}");

        let symlink_name = format!("{segment}--{version}.{extension}");

        let file_name = format!("{url_hash}--{segment}");

        let cache_dir_path = context.homebrew_dirs.cache_dir();

        let file_path = cache_dir_path.join("Cask/downloads").join(file_name);

        let symlink_path = cache_dir_path.join("Cask").join(symlink_name);

        let temp_writer_input = TempWriterInput::new(file_path, Some(symlink_path));

        Ok(temp_writer_input)
    }
}

impl PreparedCask {
    pub(super) fn url(&self) -> &str {
        &self.url
    }
}
