use serde::Serialize;

#[derive(Serialize)]
struct CaskReceipt {
    installed_on_request: bool,
    source: Source,
}

impl CaskReceipt {
    fn new(installed_on_request: bool, version: String) -> Self {
        Self {
            installed_on_request,
            source: Source {
                version,
            },
        }
    }
}

#[derive(Serialize)]
struct Source {
    version: String,
}
