use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::Path,
};

use lazy_regex::{regex, regex_captures};

pub(crate) trait PathExt {
    fn base(&self) -> anyhow::Result<&Self>;

    fn compound_extension(&self) -> Option<Cow<'_, OsStr>>;
}

impl PathExt for Path {
    fn base(&self) -> anyhow::Result<&Self> {
        let default = Self::new("/");

        let base = self.parent().unwrap_or(default);

        Ok(base)
    }

    fn compound_extension(&self) -> Option<Cow<'_, OsStr>> {
        let file_name = self.file_name()?;
        let file_name = file_name.to_str()?;

        if let Some((_, compound_extension)) =
            regex_captures!(r"\.[a-z0-9_]+\.bottle\.(?:\d+\.)?(tar\.gz)$", file_name)
        {
            let compound_extension = OsString::from(compound_extension);
            let compound_extension = Cow::Owned(compound_extension);

            return Some(compound_extension);
        }

        if let Some((_, compound_extension)) =
            regex_captures!(r"\.((?:tar|cpio|pax)\.(?:gz|bz2|lz|xz|zst|Z))$", file_name)
        {
            let compound_extension = OsString::from(compound_extension);
            let compound_extension = Cow::Owned(compound_extension);

            return Some(compound_extension);
        }

        let has_version_suffix = regex!(r"\b\d+\.\d+[^.]*$").is_match(file_name);

        let has_7z_extension = self.extension()? == OsStr::new("7z");

        if has_version_suffix && !has_7z_extension {
            return None;
        }

        let extension = self.extension()?;
        let extension = Cow::Borrowed(extension);

        Some(extension)
    }
}
