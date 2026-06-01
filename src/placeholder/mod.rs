use std::{path::PathBuf, sync::Arc};

use path_clean::PathClean as _;

use crate::context::{Context, dirs::ProjectDirs as _};

pub(crate) struct Placeholder {
    replacement_pairs: [(&'static str, String); 4],

    context: Arc<Context>,
}

impl Placeholder {
    pub(crate) fn new(context: Arc<Context>) -> Self {
        let homebrew_dirs = &context.homebrew_dirs;

        let replacement_pairs = [
            ("/$HOME", homebrew_dirs.home_dir()),
            ("$HOMEBREW_PREFIX", homebrew_dirs.prefix_dir()),
            ("$HOMEBREW_CELLAR", homebrew_dirs.cellar_dir()),
            ("$APPDIR", homebrew_dirs.app_dir()),
        ];
        let replacement_pairs = replacement_pairs.map(|(placeholder, replacement_path)| {
            let replacement_pstr = replacement_path.to_string_lossy();
            let replacement_pstr = replacement_pstr.into_owned();

            (placeholder, replacement_pstr)
        });

        Self {
            replacement_pairs,

            context,
        }
    }

    pub(crate) fn resolve_source(&self, pstr: &str) -> PathBuf {
        self.expand(pstr)
    }

    #[cfg(debug_assertions)]
    pub(crate) fn resolve_target(&self, pstr: &str) -> PathBuf {
        let path = self.expand(pstr);

        if path.is_relative() {
            return path;
        }

        let data_dir_path = self.context.homebrew_dirs.data_dir();

        let prefix_dir_path = self.context.homebrew_dirs.prefix_dir();

        if path.starts_with(&data_dir_path) || path.starts_with(prefix_dir_path) {
            return path;
        }

        match path.strip_prefix("/") {
            Ok(suffix_path) => data_dir_path.join(suffix_path),
            Err(_) => data_dir_path.join(path),
        }
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn resolve_target(&self, pstr: &str) -> PathBuf {
        self.expand(pstr)
    }

    fn expand(&self, pstr: &str) -> PathBuf {
        let pstr = match pstr.strip_prefix("~/") {
            Some(suffix_pstr) => format!("/$HOME/{suffix_pstr}"),
            None if pstr == "~" => "/$HOME".to_owned(),
            None => pstr.to_owned(),
        };

        let pstr = match pstr.strip_prefix("/Applications/") {
            Some(suffix_pstr) => format!("$APPDIR/{suffix_pstr}"),
            None if pstr == "/Applications" => "$APPDIR".to_owned(),
            None => pstr,
        };

        let pstr = self
            .replacement_pairs
            .iter()
            .fold(pstr, |pstr, (placeholder, replacement_pstr)| {
                pstr.replace(placeholder, replacement_pstr)
            });

        let path = PathBuf::from(pstr);

        path.clean()
    }
}
