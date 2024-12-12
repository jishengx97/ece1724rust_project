use airline_booking_system::{
    models::{
        ticket::FlightBookingRequest,
        ticket::SeatBookingRequest,
        ticket::TicketBookingRequest,
        user::{Role, UserRegistrationRequest},
    },
    services::{ticket_service::TicketService, user_service::UserService},
    utils::error::AppError,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use rand::Rng;
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

    let flight_result = sqlx::query!(
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

    // Create seat info
    let flight_id = flight_result.last_insert_id() as i32;
    for seat_number in 1..=capacity {
        sqlx::query!(
            r#"
            INSERT INTO seat_info (flight_id, seat_number, seat_status, version)
            VALUES (?, ?, 'AVAILABLE', 0)
            "#,
            flight_id,
            seat_number
        )
        .execute(&ctx.pool)
        .await?;
    }

    Ok(())
}

#[test_context(TicketServiceContext)]
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
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
    let booking_request = vec![FlightBookingRequest {
        flight_number,
        flight_date,
        preferred_seat: None,
    }];

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
            let result = ticket_service
                .book_ticket(user_id, TicketBookingRequest { flights: request })
                .await;
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
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
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
    let booking_request = vec![FlightBookingRequest {
        flight_number,
        flight_date,
        preferred_seat: None,
    }];

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
            let result = ticket_service
                .book_ticket(user_id, TicketBookingRequest { flights: request })
                .await;
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

#[test_context(TicketServiceContext)]
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_concurrent_seat_booking1(ctx: &TicketServiceContext) -> Result<(), AppError> {
    let test_name = "test_concurrent_seat_booking1";

    // Setup: Create a flight with capacity 10
    let flight_number = 20;
    let capacity = 10;
    let num_users = 10;
    let flight_date = NaiveDate::from_ymd_opt(2024, 12, 08).unwrap();
    let target_seat = 1; // The seat everyone will try to book

    // Setup database
    setup_database(ctx, flight_number, capacity, flight_date).await?;

    // Get flight_id
    let flight_id = sqlx::query!(
        "SELECT flight_id FROM flight WHERE flight_number = ? AND flight_date = ?",
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?
    .flight_id;

    // Register users and book tickets (without seats)
    test_println!(
        test_name,
        "Registering {} users and booking tickets...",
        num_users
    );
    let mut user_ids = Vec::new();
    for i in 0..num_users {
        // Register user
        let user = UserRegistrationRequest {
            username: format!("seat_test1_user_{}", i),
            password: "test_password".to_string(),
            role: Role::User,
            name: format!("Test User {}", i),
            birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            gender: "male".to_string(),
        };
        let user_id = ctx.user_service.register_user(user).await?;

        // Book ticket (without seat)
        let booking_request = vec![FlightBookingRequest {
            flight_number,
            flight_date,
            preferred_seat: None,
        }];
        ctx.ticket_service
            .book_ticket(
                user_id,
                TicketBookingRequest {
                    flights: booking_request,
                },
            )
            .await?;

        user_ids.push(user_id);
    }

    // Now try to book the same seat concurrently
    test_println!(test_name, "Starting concurrent seat booking attempts...");
    let seat_request = SeatBookingRequest {
        flight_number: flight_number,
        flight_date,
        seat_number: target_seat,
    };

    // Prepare all tasks
    let mut tasks = Vec::new();
    for user_id in user_ids {
        let ticket_service = ctx.ticket_service.clone();
        let request = seat_request.clone();
        tasks.push((user_id, ticket_service, request));
    }

    // Now spawn all tasks at once
    let mut join_set = JoinSet::new();
    for (user_id, ticket_service, request) in tasks {
        join_set.spawn(async move {
            let result = ticket_service.book_seat_for_ticket(user_id, request).await;
            (user_id, result)
        });
    }

    let mut successful_bookings = 0;
    while let Some(result) = join_set.join_next().await {
        match result.unwrap() {
            (user_id, Ok(_)) => {
                successful_bookings += 1;
                test_println!(
                    test_name,
                    "User {} successfully booked seat {}",
                    user_id,
                    target_seat
                );
            }
            (user_id, Err(e)) => {
                test_println!(test_name, "User {} failed to book seat: {}", user_id, e);
            }
        }
    }

    // Assert that only one booking was successful
    assert_eq!(
        successful_bookings, 1,
        "Only one seat booking should succeed"
    );

    // Verify final state
    let final_seat = sqlx::query!(
        r#"
        SELECT seat_status, version
        FROM seat_info
        WHERE flight_id = ? AND seat_number = ?
        "#,
        flight_id,
        target_seat
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        final_seat.seat_status, "BOOKED",
        "Seat should be marked as booked"
    );

    let booked_tickets = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM ticket
        WHERE flight_id = ? AND seat_number = ?
        "#,
        flight_id,
        target_seat
    )
    .fetch_one(&ctx.pool)
    .await?;

    assert_eq!(
        booked_tickets.count, 1,
        "Only one ticket should have this seat number"
    );

    Ok(())
}

#[test_context(TicketServiceContext)]
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_concurrent_seat_booking5(ctx: &TicketServiceContext) -> Result<(), AppError> {
    let test_name = "test_concurrent_seat_booking5";

    // Setup: Create a flight with capacity 10
    let flight_number = 25;
    let capacity = 30;
    let num_users = 20;
    let flight_date = NaiveDate::from_ymd_opt(2024, 12, 08).unwrap();
    let target_seats = vec![1, 2, 3, 4, 5]; // The seat everyone will try to book

    // Setup database
    setup_database(ctx, flight_number, capacity, flight_date).await?;

    // Get flight_id
    let flight_id = sqlx::query!(
        "SELECT flight_id FROM flight WHERE flight_number = ? AND flight_date = ?",
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?
    .flight_id;

    // Register users and book tickets (without seats)
    test_println!(
        test_name,
        "Registering {} users and booking tickets...",
        num_users
    );
    let mut user_ids = Vec::new();
    for i in 0..num_users {
        // Register user
        let user = UserRegistrationRequest {
            username: format!("seat_test5_user_{}", i),
            password: "test_password".to_string(),
            role: Role::User,
            name: format!("Test User {}", i),
            birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            gender: "male".to_string(),
        };
        let user_id = ctx.user_service.register_user(user).await?;

        // Book ticket (without seat)
        let booking_request = vec![FlightBookingRequest {
            flight_number,
            flight_date,
            preferred_seat: None,
        }];
        ctx.ticket_service
            .book_ticket(
                user_id,
                TicketBookingRequest {
                    flights: booking_request,
                },
            )
            .await?;

        user_ids.push(user_id);
    }

    // Now try to book the same seat concurrently
    test_println!(test_name, "Starting concurrent seat booking attempts...");
    // Prepare all tasks
    let mut tasks = Vec::new();
    for user_id in user_ids {
        // Each user randomly picks one of the target seats
        let target_seat = target_seats[rand::thread_rng().gen_range(0..target_seats.len())];
        let seat_request = SeatBookingRequest {
            flight_number: flight_number,
            flight_date,
            seat_number: target_seat,
        };
        let ticket_service = ctx.ticket_service.clone();
        tasks.push((user_id, ticket_service, seat_request));
    }

    // Now spawn all tasks at once
    let mut join_set = JoinSet::new();
    for (user_id, ticket_service, request) in tasks {
        join_set.spawn(async move {
            let result = ticket_service
                .book_seat_for_ticket(user_id, request.clone())
                .await;
            (user_id, request.seat_number, result)
        });
    }

    let mut successful_bookings = 0;
    let mut booked_seats = std::collections::HashSet::new();
    while let Some(result) = join_set.join_next().await {
        match result.unwrap() {
            (user_id, seat_number, Ok(_)) => {
                successful_bookings += 1;
                booked_seats.insert(seat_number);
                test_println!(
                    test_name,
                    "User {} successfully booked seat {}",
                    user_id,
                    seat_number
                );
            }
            (user_id, seat_number, Err(e)) => {
                test_println!(
                    test_name,
                    "User {} failed to book seat {}: {}",
                    user_id,
                    seat_number,
                    e
                );
            }
        }
    }

    // Assert that only five bookings were successful
    assert_eq!(
        successful_bookings, 5,
        "Only five seat bookings should succeed"
    );

    // Verify that all five target seats were booked
    for seat_number in &target_seats {
        let seat_info = sqlx::query!(
            r#"
            SELECT seat_status
            FROM seat_info
            WHERE flight_id = ? AND seat_number = ?
            "#,
            flight_id,
            seat_number
        )
        .fetch_one(&ctx.pool)
        .await?;

        assert_eq!(
            seat_info.seat_status, "BOOKED",
            "Seat {} should be marked as booked",
            seat_number
        );
    }

    // Verify that each seat was booked exactly once
    for seat_number in &target_seats {
        let booked_tickets = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM ticket
            WHERE flight_id = ? AND seat_number = ?
            "#,
            flight_id,
            seat_number
        )
        .fetch_one(&ctx.pool)
        .await?;

        assert_eq!(
            booked_tickets.count, 1,
            "Seat {} should be booked exactly once",
            seat_number
        );
    }

    Ok(())
}

#[test_context(TicketServiceContext)]
#[tokio::test]
async fn test_get_booking_history(ctx: &TicketServiceContext) -> Result<(), AppError> {
    // Create test user
    let user = UserRegistrationRequest {
        username: "history_test_user".to_string(),
        password: "test_password".to_string(),
        role: Role::User,
        name: "History Test User".to_string(),
        birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        gender: "male".to_string(),
    };
    let user_id = ctx.user_service.register_user(user).await?;

    // Create two different flights
    let flight_number1 = 301;
    let flight_number2 = 302;
    let capacity = 10;
    let flight_date1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let flight_date2 = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();

    // Setup database
    setup_database(ctx, flight_number1, capacity, flight_date1).await?;
    setup_database(ctx, flight_number2, capacity, flight_date2).await?;

    // Book tickets for two different flights
    let booking_request1 = vec![FlightBookingRequest {
        flight_number: flight_number1,
        flight_date: flight_date1,
        preferred_seat: Some(1),
    }];

    let booking_request2 = vec![FlightBookingRequest {
        flight_number: flight_number2,
        flight_date: flight_date2,
        preferred_seat: None,
    }];

    // Book tickets
    ctx.ticket_service
        .book_ticket(
            user_id,
            TicketBookingRequest {
                flights: booking_request1,
            },
        )
        .await?;
    ctx.ticket_service
        .book_ticket(
            user_id,
            TicketBookingRequest {
                flights: booking_request2,
            },
        )
        .await?;

    // Get booking history
    let history = ctx.ticket_service.get_history(user_id).await?;

    // Assert
    assert_eq!(history.flights.len(), 2, "Should have 2 flight bookings");

    let first_booking = &history.flights[0];
    let second_booking = &history.flights[1];

    assert_eq!(first_booking.flight_number, flight_number2);
    assert_eq!(first_booking.flight_date, flight_date2);
    assert_eq!(first_booking.seat_number, "Not Selected");
    assert_eq!(first_booking.departure_city, "New York");
    assert_eq!(first_booking.destination_city, "London");

    assert_eq!(second_booking.flight_number, flight_number1);
    assert_eq!(second_booking.flight_date, flight_date1);
    assert_eq!(second_booking.seat_number, "1");
    assert_eq!(second_booking.departure_city, "New York");
    assert_eq!(second_booking.destination_city, "London");

    Ok(())
}
