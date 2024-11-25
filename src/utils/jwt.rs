use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use serde::{Deserialize, Serialize};
use std::env;
use rocket_okapi::request::OpenApiFromRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,  // user_id
    pub exp: usize,
}

#[derive(Debug, OpenApiFromRequest)]
pub struct AuthenticatedUser {
    pub user_id: i32,
}


pub fn generate_token(user_id: i32) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        // Set expiration time to 24 hours
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiration,
    };

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = match request.headers().get_one("Authorization") {
            Some(token) if token.starts_with("Bearer ") => token[7..].to_string(),
            _ => return Outcome::Error((Status::Unauthorized, ())),
        };

        let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let token_data = match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            Ok(token) => token,
            Err(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        Outcome::Success(AuthenticatedUser {
            user_id: token_data.claims.sub,
        })
    }
}