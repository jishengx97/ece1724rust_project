use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, JsonSchema, PartialEq)]
pub enum Role {
    User,
    Admin,
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "user" => Ok(Role::User),
            "admin" => Ok(Role::Admin),
            _ => Err(serde::de::Error::custom("Invalid role: must be 'user' or 'admin'")),
        }
    }
}

impl Default for Role {
    fn default() -> Self {
        Role::User
    }
}

#[derive(Debug, Validate, Deserialize, JsonSchema)]
pub struct UserRegistrationRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub birth_date: NaiveDate,
    #[validate(custom(function = "validate_gender"))]
    pub gender: String,
    #[serde(default)]
    pub role: Role,
}

fn validate_gender(gender: &str) -> Result<(), ValidationError> {
    if gender.eq("male") || gender.eq("female") {
        Ok(())
    } else {
        Err(ValidationError::new(
            "Invalid gender: choose between male or female.",
        ))
    }
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
