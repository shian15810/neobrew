use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::Path,
};

use anyhow::{Context as _, Result};
use lazy_regex::{regex, regex_captures};

pub(crate) trait PathExt {
    fn base(&self) -> Result<&Self>;

    fn compound_extension(&self) -> Option<Cow<'_, OsStr>>;
}

impl PathExt for Path {
    fn base(&self) -> Result<&Self> {
        let base = self.parent().context("No parent directory found")?;

        Ok(base)
    }

    fn compound_extension(&self) -> Option<Cow<'_, OsStr>> {
        let name = self.file_name()?.to_str()?;

        if let Some((_, compound_extension)) =
            regex_captures!(r"\.[a-z0-9_]+\.bottle\.(?:\d+\.)?(tar\.gz)$", name)
        {
            let compound_extension = OsString::from(compound_extension);
            let compound_extension = Cow::Owned(compound_extension);

            return Some(compound_extension);
        }

        if let Some((_, compound_extension)) =
            regex_captures!(r"\.((?:tar|cpio|pax)\.(?:gz|bz2|lz|xz|zst|Z))$", name)
        {
            let compound_extension = OsString::from(compound_extension);
            let compound_extension = Cow::Owned(compound_extension);

            return Some(compound_extension);
        }

        if regex!(r"\b\d+\.\d+[^.]*$").is_match(name) && self.extension()? != OsStr::new("7z") {
            return None;
        }

        let extension = self.extension()?;
        let extension = Cow::Borrowed(extension);

        Some(extension)
    }
}
