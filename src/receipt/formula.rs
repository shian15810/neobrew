use serde::Serialize;

#[derive(Serialize)]
struct FormulaReceipt {
    installed_on_request: bool,
    source: Source,
}

impl FormulaReceipt {
    fn new(installed_on_request: bool, version: String) -> Self {
        Self {
            installed_on_request,
            source: Source {
                versions: Versions {
                    stable: version,
                },
            },
        }
    }
}

#[derive(Serialize)]
struct Source {
    versions: Versions,
}

#[derive(Serialize)]
struct Versions {
    stable: String,
}
