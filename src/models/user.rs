use serde::{Deserialize, Serialize}; 
use schemars::JsonSchema;

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CustomerInfo {
    pub id: i32,
    pub name: String,
    pub gender: Gender,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "ENUM")]
pub enum Gender {
    #[sqlx(rename = "male")]
    Male,
    #[sqlx(rename = "female")]
    Female,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserRegistrationRequest {
    pub username: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UserLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UserLoginResponse {
    pub token: String,
    pub user_id: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(example = "RegisterResponse::example")]
pub struct RegisterResponse {
    #[schemars(title = "User ID")]
    pub user_id: i32,
    
    #[schemars(title = "Register Status")]
    pub status: String,
}

impl RegisterResponse {
    pub fn example() -> Self {
        Self {
            user_id: 123,
            status: "success".to_string(),
        }
    }
}
