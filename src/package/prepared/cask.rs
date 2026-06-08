use super::{
    super::{
        Packageable,
        raw::{Artifact, Variation},
        resolved::ResolvedCask,
    },
    PreparedPackageable,
    cask_stanza::Stanzas,
};

pub(crate) struct PreparedCask {
    pub(in super::super) token: String,
    pub(in super::super) version: String,
    variation_tag: Option<String>,
    variation_url: String,
    variation_sha256: String,
    variation_stanzas: Stanzas,
    pub(in super::super) is_requested: bool,
}

impl TryFrom<(ResolvedCask, bool)> for PreparedCask {
    type Error = Option<anyhow::Error>;

    fn try_from((resolved_cask, is_requested): (ResolvedCask, bool)) -> Result<Self, Self::Error> {
        let token = resolved_cask.token.clone();

        let version = resolved_cask.version.clone();

        let (variation_tag, variation) = resolved_cask.variation_entry()?;

        let this = Self {
            token,
            version,
            variation_tag,
            variation_url: variation.url,
            variation_sha256: variation.sha256,
            variation_stanzas: Stanzas::from(variation.artifacts),
            is_requested,
        };

        Ok(this)
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
    fn download_url(&self) -> &str {
        &self.variation_url
    }

    fn expected_sha256(&self) -> &str {
        &self.variation_sha256
    }
}

impl PreparedCask {
    pub(crate) fn stanzas(&self) -> &Stanzas {
        &self.variation_stanzas
    }
}

impl ResolvedCask {
    fn variation_entry(mut self) -> anyhow::Result<(Option<String>, Variation<Vec<Artifact>>)> {
        #[expect(clippy::collapsible_if)]
        if let Some(tag) = self.variation_tag()? {
            if let Some((tag, variation)) = self.variations.remove_entry(&tag) {
                let variation = variation.unwrap_artifacts_or(self.artifacts);

                let entry = (Some(tag), variation);

                return Ok(entry);
            }
        }

        let variation = Variation {
            url: self.url,
            sha256: self.sha256,
            artifacts: self.artifacts,
        };

        let entry = (None, variation);

        Ok(entry)
    }

    #[cfg(target_os = "macos")]
    fn variation_tag(&self) -> anyhow::Result<Option<String>> {
        use crate::{ext::core::result::ResultExt as _, util::macos};

        let current_macos_tag = macos::Tag::try_default()?;

        #[cfg(debug_assertions)]
        let tagged_candidate_macos_tags = self
            .variations
            .keys()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .try_collect::<Vec<_>>()?;

        #[cfg(not(debug_assertions))]
        let tagged_candidate_macos_tags = self
            .variations
            .keys()
            .map(|tag| tag.parse::<macos::Tag>().map(|macos_tag| (tag, macos_tag)))
            .filter_map(Result::transpose_err)
            .collect::<anyhow::Result<Vec<_>>>()?;

        let tag = tagged_candidate_macos_tags
            .into_iter()
            .filter(|(_, candidate_macos_tag)| {
                let is_macos_architecture_equal =
                    candidate_macos_tag.architecture() == current_macos_tag.architecture();

                is_macos_architecture_equal && candidate_macos_tag <= &current_macos_tag
            })
            .max_by(|(_, left), (_, right)| left.cmp(right))
            .map(|(tag, _)| tag.to_owned());

        let Some(tag) = tag else {
            return Ok(None);
        };

        Ok(Some(tag))
    }

    #[cfg(target_os = "linux")]
    #[expect(clippy::unnecessary_wraps)]
    fn variation_tag(&self) -> anyhow::Result<Option<String>> {
        let tag = cfg_select! {
            target_arch = "aarch64" => "arm64_linux",
            target_arch = "x86_64" => "x86_64_linux",
        };
        let tag = self.variations.contains_key(tag).then(|| tag.to_owned());

        let Some(tag) = tag else {
            return Ok(None);
        };

        Ok(Some(tag))
    }
}

impl Variation {
    fn unwrap_artifacts_or(self, default: Vec<Artifact>) -> Variation<Vec<Artifact>> {
        #[cfg(debug_assertions)]
        let this = Variation {
            artifacts: self.artifacts.unwrap_or(default),

            ..self
        };

        #[cfg(not(debug_assertions))]
        let this = Variation {
            url: self.url,
            sha256: self.sha256,
            artifacts: self.artifacts.unwrap_or(default),
        };

        this
    }
}
