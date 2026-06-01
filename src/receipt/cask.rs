use serde::Serialize;

#[derive(Serialize)]
struct CaskReceipt {
    installed_on_request: bool,
}

impl CaskReceipt {
    fn new(installed_on_request: bool) -> Self {
        Self {
            installed_on_request,
        }
    }
}
