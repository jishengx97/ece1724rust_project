use thiserror::Error;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::Request;
use rocket::Response;
use rocket::http::ContentType;
use std::io::Cursor;
use serde_json::json;
use serde::Serialize;
use rocket_okapi::JsonSchema;

#[derive(Error, Debug, Serialize, JsonSchema)]
pub enum AppError {
    #[error("Database error")]
    DatabaseError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable: {0}")]
    Unprocessable(String),

    #[error("Bad request: {0}")]
    BadRequest(String),
}

// Convert sqlx::Error (database error) to AppError::DatabaseError
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

// Define a type alias for the result type
pub type AppResult<T> = Result<T, AppError>;

// Implement the Responder trait for AppError
// Format all error from route level to a Http Response at route level
#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = match self {
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::NotFound(_) => Status::NotFound,
            AppError::DatabaseError(_) => Status::InternalServerError,
            AppError::AuthError(_) => Status::Unauthorized,
            AppError::Conflict(_) => Status::Conflict,
            AppError::Unprocessable(_) => Status::UnprocessableEntity,
            AppError::BadRequest(_) => Status::BadRequest,
        };

        let json = json!({
            "error": self.to_string()
        });

        Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(None, Cursor::new(json.to_string()))
            .ok()
    }
}

