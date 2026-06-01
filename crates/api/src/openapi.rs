use utoipa::{
    OpenApi,
    openapi::{RefOr, path::Operation},
    // openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};

#[utoipauto::utoipauto(paths = "./api/src from crate::routes")]
#[derive(OpenApi)]
#[openapi(
    // paths(),
    // components(),
    // modifiers(&SecurityAddon),
    // info(title = "API", version = "1.0.0"),
    modifiers(&Modifier),
)]
pub struct ApiDoc;

#[expect(clippy::unwrap_used)]
pub fn print_openapi() {
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
}

struct Modifier;

impl utoipa::Modify for Modifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.paths.paths.iter_mut().for_each(|path| {
            if let Some(operation) = &mut path.1.get {
                insert_responses(operation);
            }
            if let Some(operation) = &mut path.1.patch {
                insert_responses(operation);
            }
            if let Some(operation) = &mut path.1.post {
                insert_responses(operation);
            }
            if let Some(operation) = &mut path.1.delete {
                insert_responses(operation);
            }
        });
    }
}

fn insert_responses(operation: &mut Operation) {
    operation.responses.responses.insert(
        String::from("400"),
        RefOr::T(utoipa::openapi::Response::new("Bad request")),
    );
    operation.responses.responses.insert(
        String::from("401"),
        RefOr::T(utoipa::openapi::Response::new("Unauthorized")),
    );
    operation.responses.responses.insert(
        String::from("403"),
        RefOr::T(utoipa::openapi::Response::new("Forbidden")),
    );
    operation.responses.responses.insert(
        String::from("500"),
        RefOr::T(utoipa::openapi::Response::new("Internal Server Error")),
    );
}
