use crate::models::user::{User, UserLoginRequest, UserLoginResponse, UserRegistrationRequest, Role};
use crate::utils::error::{AppError, AppResult};
use crate::utils::jwt;
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::MySqlPool;
use validator::Validate;

#[derive(Clone)]
pub struct UserService {
    pool: MySqlPool,
}

impl UserService {
    pub fn new(pool: MySqlPool) -> Self {
        UserService { pool }
    }

    // Register a new user
    pub async fn register_user(&self, request: UserRegistrationRequest) -> AppResult<i32> {
        // Validate the request
        request
            .validate()
            .map_err(|e| AppError::ValidationError(format!("{:?}", e)))?;

        // Check if username already exists
        let existing_user =
            sqlx::query!("SELECT id FROM user WHERE username = ?", request.username)
                .fetch_optional(&self.pool)
                .await?;

        if existing_user.is_some() {
            return Err(AppError::Conflict("Username already exists".into()));
        }

        // Hash password
        let hashed_password = hash(request.password.as_bytes(), DEFAULT_COST)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        // Convert role to string for database insertion
        let role_str = match request.role {
            Role::Admin => "ADMIN",
            Role::User => "USER",
        };

        // Insert user with role
        let result = sqlx::query!(
            "INSERT INTO user (username, password, role) VALUES (?, ?, ?)",
            request.username,
            hashed_password,
            role_str
        )
        .execute(&self.pool)
        .await?;

        // Insert customer info to customer_info table
        let _customer_info_result = sqlx::query!(
            "INSERT INTO customer_info (id, name, birth_date, gender) 
            VALUES(?, ?, ?, ?)",
            result.last_insert_id(),
            request.name,
            request.birth_date,
            request.gender,
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
        let token = jwt::generate_token(user.id).map_err(|e| AppError::AuthError(e.to_string()))?;

        Ok(UserLoginResponse {
            token,
            user_id: user.id,
        })
    }
}
