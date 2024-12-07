// Just duplicate the test from user_service_test.rs to test multiple test in different classes
// TODO: Delete this test after we add more tests
use airline_booking_system::{
    models::user::{Role, UserRegistrationRequest},
    services::user_service::UserService,
    utils::error::AppError,
};
use chrono::NaiveDate;
use sqlx::mysql::MySqlPool as Pool;
use test_context::{AsyncTestContext, test_context};
use async_trait::async_trait;
mod common {
    pub mod test_utils;
}
use common::test_utils::TestDb;
use ctor::dtor;

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
            
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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

// Just duplicate the test above to test multiple test in one class
#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_registration_success_two(ctx: &UserServiceContext) -> Result<(), AppError> {
    // Test Data
    let test_user = UserRegistrationRequest {
        username: "test_user_registration_duplicate".to_string(),
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