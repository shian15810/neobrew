use std::sync::Arc;

use foyer::{Cache, CacheBuilder};
use url::Url;

use crate::package::Formula;

struct FormulaRegistry {
    store: Cache<String, Arc<Formula>>,

    base_url: Url,
    json_url: Url,
    jws_json_url: Url,
    tap_migrations_url: Url,
    tap_migrations_jws_url: Url,
}

impl FormulaRegistry {
    const BASE_URL: &str = "https://formulae.brew.sh/api/formula/";
    const JSON_URL: &str = "https://formulae.brew.sh/api/formula.json";
    const JWS_JSON_URL: &str = "https://formulae.brew.sh/api/formula.jws.json";
    const TAP_MIGRATIONS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.json";
    #[rustfmt::skip]
    const TAP_MIGRATIONS_JWS_URL: &str = "https://formulae.brew.sh/api/formula_tap_migrations.jws.json";

    fn new() -> Self {
        Self {
            store: CacheBuilder::new(usize::MAX).build(),

            base_url: Url::parse(Self::BASE_URL).unwrap(),
            json_url: Url::parse(Self::JSON_URL).unwrap(),
            jws_json_url: Url::parse(Self::JWS_JSON_URL).unwrap(),
            tap_migrations_url: Url::parse(Self::TAP_MIGRATIONS_URL).unwrap(),
            tap_migrations_jws_url: Url::parse(Self::TAP_MIGRATIONS_JWS_URL).unwrap(),
        }
    }
}
