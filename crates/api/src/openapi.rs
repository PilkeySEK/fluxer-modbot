use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};

#[utoipauto::utoipauto(paths = "./api/src from crate::routes")]
#[derive(OpenApi)]
#[openapi(
    // paths(),
    // components(),
    // modifiers(&SecurityAddon),
    // info(title = "API", version = "1.0.0"),
)]
pub struct ApiDoc;

pub fn print_openapi() {
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
}
