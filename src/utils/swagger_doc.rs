use crate::utils::error::AppError;
use rocket_okapi::okapi::openapi3::{Response, Responses, MediaType};
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::RefOr;
use okapi::openapi3::SchemaObject;
use indexmap::IndexMap;
use serde_json::json;
use rocket::http::Status;

impl<'r> OpenApiResponderInner for AppError {
    fn responses(gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        
        // Define error responses
        let error_responses = [
            (Status::BadRequest, "Bad Request", AppError::ValidationError("Bad Requests".to_string())),
            (Status::Unauthorized, "Unauthorized", AppError::AuthError("Unauthorized".to_string())),
            (Status::NotFound, "NotFound", AppError::NotFound("Not Found".to_string())),
            (Status::Conflict, "Conflict", AppError::Conflict("Conflict".to_string())),
            (Status::InternalServerError, "InternalServerError", AppError::DatabaseError("Internal ServerError".to_string())),
            (Status::UnprocessableEntity, "Unprocessable", AppError::Unprocessable("Unprocessable".to_string())),
        ];

        for (status, description, error) in error_responses {
            responses.responses.insert(
                status.code.to_string(),
                RefOr::Object(Response {
                    description: description.to_string(),
                    content: {
                        let mut content = IndexMap::new();
                        content.insert(
                            "application/json".to_string(),
                            MediaType {
                                schema: Some(SchemaObject::default()),
                                example: Some(json!({
                                    "error": error.to_string()
                                })),
                                ..Default::default()
                            },
                        );
                        content
                    },
                    ..Default::default()
                }),
            );
        }
        
        Ok(responses)
    }
}