use std::sync::Arc;

use oci_client::{Reference, manifest::OciDescriptor};

use crate::{
    context::Context,
    package::prepared::{PreparedFormula, PreparedPackageable as _},
};

pub(super) struct FormulaStream {
    context: Arc<Context>,
}

impl FormulaStream {
    pub(super) const OCI_REGISTRY_URL: &str = "ghcr.io";

    pub(super) fn new(context: Arc<Context>) -> Self {
        Self {
            context,
        }
    }

    #[expect(clippy::unused_self)]
    pub(super) fn oci(
        &self,
        prepared_formula: &PreparedFormula,
    ) -> Option<(Reference, OciDescriptor)> {
        let registry = Self::OCI_REGISTRY_URL;

        let url = prepared_formula.download_url();

        let repository = format!("https://{registry}/v2/");
        let repository = url.strip_prefix(&repository)?;

        let (repository, _) = repository.split_once("/blobs/")?;

        let sha256 = prepared_formula.expected_sha256();

        let digest = format!("sha256:{sha256}");

        let reference =
            Reference::with_digest(registry.to_owned(), repository.to_owned(), digest.clone());

        let descriptor = OciDescriptor {
            digest,

            ..OciDescriptor::default()
        };

        Some((reference, descriptor))
    }
}
