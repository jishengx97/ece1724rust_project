use airline_booking_system::{
    models::user::{Role, UserRegistrationRequest, UserLoginRequest},
    services::user_service::UserService,
    utils::error::AppError,
};
use chrono::NaiveDate;
use sqlx::mysql::MySqlPool as Pool;
use test_context::{AsyncTestContext, test_context};
use async_trait::async_trait;
use ctor::dtor;

mod common {
    pub mod test_utils;
}
use common::test_utils::TestDb;

struct UserServiceContext {
    pool: Pool,
    user_service: UserService,
}

// Teardown function to clean up the test database after all tests are run
#[dtor]
fn cleanup() {
    if let Err(e) = TestDb::cleanup_database_sync() {
        eprintln!("Failed to cleanup test database: {}", e);
    }
}

// Setup function to initialize the test database for each test and teardown function to drop connections after each test
#[async_trait]
impl AsyncTestContext for UserServiceContext {
    // Setup function to initialize the test database for each test
    async fn setup() -> Self {
        let pool = TestDb::get_instance()
            .await
            .expect("Failed to get test database instance");
            
        let user_service = UserService::new(pool.clone());
        
        UserServiceContext {
            pool,
            user_service,
        }
    }

    // Teardown function to drop connections after each test
    async fn teardown(self) {
        let _ = sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await;

        //self.pool.close().await;
    }
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_registration_success(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Test Data
    let test_user = UserRegistrationRequest {
        username: "test_user_registration".to_string(),
        password: "test_password123".to_string(),
        role: Role::User,
        name: "Test User".to_string(),
        birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        gender: "male".to_string(),
    };
    
    let expected_username = test_user.username.clone();
    let expected_name = test_user.name.clone();
    let expected_gender = test_user.gender.clone();
    
    // Register user
    let user_id = ctx.user_service.register_user(test_user).await?;
    
    // Assert
    assert!(user_id > 0, "User ID should be positive");
    
    let saved_user = sqlx::query!(
        r#"
        SELECT u.username, u.role, c.name, c.gender 
        FROM user u 
        JOIN customer_info c ON u.id = c.id 
        WHERE u.id = ?
        "#,
        user_id
    )
    .fetch_one(&ctx.pool)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    assert_eq!(saved_user.username, expected_username);
    assert_eq!(saved_user.role, "USER");
    assert_eq!(saved_user.name, expected_name);
    assert_eq!(saved_user.gender, expected_gender);
    
    Ok(())
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_registration_duplicate_username(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Create a user with the same username
    let existing_username = "duplicate_test_user";
    let hashed_password = bcrypt::hash("existing_password", bcrypt::DEFAULT_COST).unwrap();
    
    sqlx::query!(
        "INSERT INTO user (username, password, role) VALUES (?, ?, ?)",
        existing_username,
        hashed_password,
        "USER"
    )
    .execute(&ctx.pool)
    .await?;

    // Try to register a new user with the same username by user service
    let test_user = UserRegistrationRequest {
        username: existing_username.to_string(),
        password: "new_password123".to_string(),
        role: Role::User,
        name: "Test User".to_string(),
        birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        gender: "male".to_string(),
    };
    
    let result = ctx.user_service.register_user(test_user).await;
    
    // Assert
    match result {
        Err(AppError::Conflict(msg)) => {
            assert_eq!(msg, "Username already exists");
            Ok(())
        }
        _ => panic!("Expected Conflict error for duplicate username"),
    }
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_login_success(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Create a test user directly in the database
    let test_username = "login_test_user";
    let test_password = "test_password123";
    let hashed_password = bcrypt::hash(test_password, bcrypt::DEFAULT_COST).unwrap();
    
    sqlx::query!(
        "INSERT INTO user (username, password, role) VALUES (?, ?, ?)",
        test_username,
        hashed_password,
        "USER"
    )
    .execute(&ctx.pool)
    .await?;

    // Attempt to login
    let login_request = UserLoginRequest {
        username: test_username.to_string(),
        password: test_password.to_string(),
    };

    let login_response = ctx.user_service.login_user(login_request).await?;

    // Assert
    assert!(login_response.user_id > 0, "User ID should be positive");
    assert!(!login_response.token.is_empty(), "Token should not be empty");
    
    Ok(())
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_login_nonexistent_username(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Create a different test user
    let test_username = "another_test_user";
    let test_password = "test_password123";
    let hashed_password = bcrypt::hash(test_password, bcrypt::DEFAULT_COST).unwrap();
    
    sqlx::query!(
        "INSERT INTO user (username, password, role) VALUES (?, ?, ?)",
        test_username,
        hashed_password,
        "USER"
    )
    .execute(&ctx.pool)
    .await?;

    // Attempt to login with non-existent username
    let login_request = UserLoginRequest {
        username: "nonexistent_user".to_string(),
        password: "some_password".to_string(),
    };

    let result = ctx.user_service.login_user(login_request).await;

    // Assert
    match result {
        Err(AppError::AuthError(msg)) => {
            assert_eq!(msg, "Invalid credentials");
            Ok(())
        }
        _ => panic!("Expected AuthError for non-existent username"),
    }
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_login_wrong_password(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Create another test user
    let test_username = "password_test_user";
    let test_password = "correct_password";
    let hashed_password = bcrypt::hash(test_password, bcrypt::DEFAULT_COST).unwrap();
    
    sqlx::query!(
        "INSERT INTO user (username, password, role) VALUES (?, ?, ?)",
        test_username,
        hashed_password,
        "USER"
    )
    .execute(&ctx.pool)
    .await?;

    // Attempt to login with wrong password
    let login_request = UserLoginRequest {
        username: test_username.to_string(),
        password: "wrong_password".to_string(),
    };

    let result = ctx.user_service.login_user(login_request).await;

    // Assert
    match result {
        Err(AppError::AuthError(msg)) => {
            assert_eq!(msg, "Invalid credentials");
            Ok(())
        }
        _ => panic!("Expected AuthError for wrong password"),
    }
}