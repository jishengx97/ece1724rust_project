use airline_booking_system::{
    models::user::{Role, UserRegistrationRequest},
    services::user_service::UserService,
    utils::error::AppError,
};
use chrono::NaiveDate;
use sqlx::mysql::MySqlPool as Pool;
use test_context::{AsyncTestContext, test_context};
use async_trait::async_trait;

mod test_utils;
use test_utils::TestDb;

use ctor::dtor;

struct UserServiceContext {
    pool: Pool,
    user_service: UserService,
}

#[async_trait]
impl AsyncTestContext for UserServiceContext {
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

    async fn teardown(self) {
    }
}

#[test_context(UserServiceContext)]
#[tokio::test]
async fn test_user_registration_success(ctx: &UserServiceContext) -> Result<(), AppError> {
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
    
    let user_id = ctx.user_service.register_user(test_user).await?;
    
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
async fn test_user_registration_success_two(ctx: &UserServiceContext) -> Result<(), AppError> {
    let test_user = UserRegistrationRequest {
        username: "test_user_registration2".to_string(),
        password: "test_password123".to_string(),
        role: Role::User,
        name: "Test User".to_string(),
        birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        gender: "male".to_string(),
    };
    
    let expected_username = test_user.username.clone();
    let expected_name = test_user.name.clone();
    let expected_gender = test_user.gender.clone();
    
    let user_id = ctx.user_service.register_user(test_user).await?;
    
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


#[dtor]
fn cleanup() {
    println!("Starting database cleanup...");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = TestDb::cleanup_database().await {
            eprintln!("Failed to cleanup test database: {}", e);
        } else {
            println!("Database cleanup completed successfully");
        }
    });
}