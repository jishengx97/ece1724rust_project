use airline_booking_system::{
    models::{
        ticket::TicketBookingRequest,
        user::{Role, UserRegistrationRequest},
    },
    services::{ticket_service::TicketService, user_service::UserService},
    utils::error::AppError,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::mysql::MySqlPool as Pool;
use test_context::{test_context, AsyncTestContext};
use tokio::task::JoinSet;

mod common {
    pub mod test_utils;
}
use common::test_utils::TestDb;
use ctor::dtor;

struct TicketServiceContext {
    pool: Pool,
    ticket_service: TicketService,
    user_service: UserService,
}

#[dtor]
fn cleanup() {
    if let Err(e) = TestDb::cleanup_database_sync() {
        eprintln!("Failed to cleanup test database: {}", e);
    }
}

#[async_trait]
impl AsyncTestContext for TicketServiceContext {
    async fn setup() -> Self {
        let pool = TestDb::get_instance(file!())
            .await
            .expect("Failed to get test database instance");

        let ticket_service = TicketService::new(pool.clone());
        let user_service = UserService::new(pool.clone());

        TicketServiceContext {
            pool,
            ticket_service,
            user_service,
        }
    }

    async fn teardown(self) {
        let _ = sqlx::query("SELECT 1").execute(&self.pool).await;
    }
}

async fn setup_database(
    ctx: &TicketServiceContext,
    flight_number: i32,
    capacity: i32,
    flight_date: NaiveDate,
) -> Result<(), AppError> {
    // use the same flight number and aircraft id because we don't care
    sqlx::query!(
        r#"INSERT INTO aircraft (aircraft_id, capacity) VALUES (?, ?)"#,
        flight_number,
        capacity
    )
    .execute(&ctx.pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO flight_route
        (flight_number, departure_city, destination_city, departure_time, arrival_time,
            aircraft_id, overbooking, start_date, end_date)
        VALUES
        (?, 'New York', 'London', '10:00:00', '22:00:00',
            ?, 0.00, ?, ?)
        "#,
        flight_number,
        flight_number,
        flight_date,
        flight_date
    )
    .execute(&ctx.pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO flight (flight_number, flight_date, available_tickets, version)
        VALUES (?, ?, ?, 1)
        "#,
        flight_number,
        flight_date,
        capacity
    )
    .execute(&ctx.pool)
    .await?;

    Ok(())
}

#[test_context(TicketServiceContext)]
#[tokio::test]
async fn test_concurrent_ticket_booking_capacity1(
    ctx: &TicketServiceContext,
) -> Result<(), AppError> {
    let test_name = "test_concurrent_ticket_booking_capacity1";
    // Setup: Create a flight with capacity 1
    let flight_number = 10;
    let capacity = 1;
    let num_users = 10;
    let flight_date = NaiveDate::from_ymd_opt(2024, 12, 08).unwrap();

    setup_database(ctx, flight_number, capacity, flight_date).await?;

    // Register 10 test users
    test_println!(test_name, "Registering {} users...", num_users);
    let mut user_ids = Vec::new();
    for i in 0..num_users {
        let user = UserRegistrationRequest {
            username: format!("concurrent1_test_user_{}", i),
            password: "test_password".to_string(),
            role: Role::User,
            name: format!("Test User {}", i),
            birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            gender: "male".to_string(),
        };
        let user_id = ctx.user_service.register_user(user).await?;
        user_ids.push(user_id);
    }

    // Create booking request
    let booking_request = TicketBookingRequest {
        flight_number,
        flight_date,
        preferred_seat: None,
    };

    test_println!(test_name, "Starting concurrent booking attempts...");
    // Prepare all tasks first
    let mut tasks = Vec::new();
    for user_id in user_ids {
        let ticket_service = ctx.ticket_service.clone();
        let request = booking_request.clone();
        tasks.push((user_id, ticket_service, request));
    }

    // Now spawn all tasks at once
    let mut join_set = JoinSet::new();
    for (user_id, ticket_service, request) in tasks {
        join_set.spawn(async move {
            let result = ticket_service.book_ticket(user_id, request).await;
            (user_id, result)
        });
    }

    let mut successful_bookings = 0;
    while let Some(result) = join_set.join_next().await {
        match result.unwrap() {
            (user_id, Ok(_)) => {
                successful_bookings += 1;
                test_println!(test_name, "User {} successfully booked the ticket", user_id);
            }
            (user_id, Err(e)) => {
                test_println!(test_name, "User {} failed to book: {}", user_id, e);
            }
        }
    }

    // Assert that only one booking was successful
    assert_eq!(successful_bookings, 1, "Only one booking should succeed");

    // Verify final state
    let final_tickets = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM ticket
        WHERE flight_number = ? AND flight_date = ?
        "#,
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        final_tickets.count, 1,
        "There should be exactly one ticket in the database"
    );

    let final_flight = sqlx::query!(
        r#"
        SELECT available_tickets
        FROM flight
        WHERE flight_number = ? AND flight_date = ?
        "#,
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        final_flight.available_tickets, 0,
        "Available tickets should be 0"
    );

    Ok(())
}

#[test_context(TicketServiceContext)]
#[tokio::test]
async fn test_concurrent_ticket_booking_capacity5(
    ctx: &TicketServiceContext,
) -> Result<(), AppError> {
    let test_name = "test_concurrent_ticket_booking_capacity5";
    // Setup: Create a flight with capacity 1
    let flight_number = 11;
    let capacity = 5;
    let num_users = 20;
    let flight_date = NaiveDate::from_ymd_opt(2024, 12, 08).unwrap();

    setup_database(ctx, flight_number, capacity, flight_date).await?;

    // Register 10 test users
    test_println!(test_name, "Registering {} users...", num_users);
    let mut user_ids = Vec::new();
    for i in 0..num_users {
        let user = UserRegistrationRequest {
            username: format!("concurrent2_test_user_{}", i),
            password: "test_password".to_string(),
            role: Role::User,
            name: format!("Test User {}", i),
            birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            gender: "male".to_string(),
        };
        let user_id = ctx.user_service.register_user(user).await?;
        user_ids.push(user_id);
    }

    // Create booking request
    let booking_request = TicketBookingRequest {
        flight_number,
        flight_date,
        preferred_seat: None,
    };

    test_println!(test_name, "Starting concurrent booking attempts...");
    // Prepare all tasks first
    let mut tasks = Vec::new();
    for user_id in user_ids {
        let ticket_service = ctx.ticket_service.clone();
        let request = booking_request.clone();
        tasks.push((user_id, ticket_service, request));
    }

    // Now spawn all tasks at once
    let mut join_set = JoinSet::new();
    for (user_id, ticket_service, request) in tasks {
        join_set.spawn(async move {
            let result = ticket_service.book_ticket(user_id, request).await;
            (user_id, result)
        });
    }

    let mut successful_bookings = 0;
    while let Some(result) = join_set.join_next().await {
        match result.unwrap() {
            (user_id, Ok(_)) => {
                successful_bookings += 1;
                test_println!(test_name, "User {} successfully booked the ticket", user_id);
            }
            (user_id, Err(e)) => {
                test_println!(test_name, "User {} failed to book: {}", user_id, e);
            }
        }
    }

    // Assert that only one booking was successful
    assert_eq!(
        successful_bookings, capacity,
        "Only {} booking should succeed",
        capacity
    );

    // Verify final state
    let final_tickets = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM ticket
        WHERE flight_number = ? AND flight_date = ?
        "#,
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        final_tickets.count, capacity as i64,
        "There should be exactly {} ticket in the database",
        capacity
    );

    let final_flight = sqlx::query!(
        r#"
        SELECT available_tickets
        FROM flight
        WHERE flight_number = ? AND flight_date = ?
        "#,
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        final_flight.available_tickets, 0,
        "Available tickets should be 0"
    );

    Ok(())
}
