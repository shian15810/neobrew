mod cask;
mod formula;

trait Registry {
    const BASE_URL: &str;
    const JSON_URL: &str;
    const JWS_JSON_URL: &str;
    const TAP_MIGRATIONS_URL: &str;
    const TAP_MIGRATIONS_JWS_URL: &str;

    fn new() -> Self;
}
