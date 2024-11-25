use crate::models::user::{
    RegisterResponse, UserLoginRequest, UserLoginResponse, UserRegistrationRequest,
};
use crate::services::user_service::UserService;
use crate::utils::error::AppError;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

/// Register a new user
#[openapi(tag = "Users")]
#[post("/register", format = "json", data = "<request>")]
pub async fn register(
    request: Json<UserRegistrationRequest>,
    user_service: &State<UserService>,
) -> Result<Json<RegisterResponse>, AppError> {
    let user_id = user_service.register_user(request.into_inner()).await?;
    Ok(Json(RegisterResponse {
        user_id,
        status: "success".to_string(),
    }))
}

/// Login a user
#[openapi(tag = "Users")]
#[post("/login", format = "json", data = "<request>")]
pub async fn login(
    request: Json<UserLoginRequest>,
    user_service: &State<UserService>,
) -> Result<Json<UserLoginResponse>, AppError> {
    let response = user_service.login_user(request.into_inner()).await?;
    Ok(Json(response))
}
