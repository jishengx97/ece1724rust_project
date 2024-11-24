use crate::models::user::{User, UserLoginRequest, UserRegistrationRequest, UserLoginResponse};
use crate::utils::error::{AppError, AppResult};
use crate::utils::jwt;
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::MySqlPool;

pub struct UserService {
    pool: MySqlPool,
}

impl UserService {
    pub fn new(pool: MySqlPool) -> Self {
        UserService { pool }
    }

    // Register a new user
    pub async fn register_user(&self, request: UserRegistrationRequest) -> AppResult<i32> {
        // Check if username already exists
        let existing_user = sqlx::query!(
            "SELECT id FROM user WHERE username = ?",
            request.username
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing_user.is_some() {
            return Err(AppError::Conflict("Username already exists".into()));
        }

        // Hash password
        let hashed_password = hash(request.password.as_bytes(), DEFAULT_COST)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        // Insert user
        let result = sqlx::query!(
            "INSERT INTO user (username, password, role) VALUES (?, ?, 'USER')",
            request.username,
            hashed_password
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    // Login user
    pub async fn login_user(&self, request: UserLoginRequest) -> AppResult<UserLoginResponse> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, username, password, role FROM user WHERE username = ?",
            request.username
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::AuthError("Invalid credentials".into()))?;

        // Verify password
        let password_matches = verify(request.password.as_bytes(), &user.password)
            .map_err(|e| AppError::AuthError(e.to_string()))?;

        if !password_matches {
            return Err(AppError::AuthError("Invalid credentials".into()));
        }

        // Generate JWT token
        let token = jwt::generate_token(user.id)
            .map_err(|e| AppError::AuthError(e.to_string()))?;

        Ok(UserLoginResponse {
            token,
            user_id: user.id,
        })
    }
}