use std::sync::Arc;

use oci_client::{Reference, manifest::OciDescriptor};

use crate::{context::Context, package::prepared::PreparedFormula};

pub(super) struct FormulaStream {
    context: Arc<Context>,
}

impl FormulaStream {
    pub(super) const OCI_REGISTRY: &str = "ghcr.io";

    pub(super) fn oci(prepared_formula: &PreparedFormula) -> Option<(Reference, OciDescriptor)> {
        let registry = Self::OCI_REGISTRY;

        let repository = format!("https://{registry}/v2/");
        let repository = prepared_formula.oci_url().strip_prefix(&repository)?;

        let (repository, _) = repository.split_once("/blobs/")?;

        let sha256 = prepared_formula.oci_sha256();

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
