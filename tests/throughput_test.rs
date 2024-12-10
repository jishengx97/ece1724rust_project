use airline_booking_system::{
    models::{
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
use sqlx::mysql::MySqlPool;
use std::time::Duration;
use std::time::Instant;
use test_context::{test_context, AsyncTestContext};
use tokio::task::JoinSet;

mod common {
    pub mod test_utils;
}
use common::test_utils::TestDb;
use ctor::dtor;

struct ThroughputContext {
    pool: MySqlPool,
    ticket_service: TicketService,
    user_service: UserService,
}

#[derive(Debug, Clone)]
enum MixedRequest {
    Booking((i32, i32, NaiveDate)),
    SeatSelection((i32, i32, NaiveDate, i32)),
}

#[dtor]
fn cleanup() {
    if let Err(e) = TestDb::cleanup_database_sync() {
        eprintln!("Failed to cleanup test database: {}", e);
    }
}

#[async_trait]
impl AsyncTestContext for ThroughputContext {
    async fn setup() -> Self {
        let pool = TestDb::get_instance(file!())
            .await
            .expect("Failed to get test database instance");

        if let Ok(row) = sqlx::query!("SELECT @@max_connections as max")
            .fetch_one(&pool)
            .await
        {
            if let Some(max) = row.max {
                test_println!("setup", "Database max connections: {}", max);
            }
        }

        let ticket_service = TicketService::new(pool.clone());
        let user_service = UserService::new(pool.clone());

        ThroughputContext {
            pool,
            ticket_service,
            user_service,
        }
    }

    async fn teardown(self) {
        let _ = sqlx::query("SELECT 1").execute(&self.pool).await;
    }
}

struct PerformanceMetrics {
    total_requests: u32,
    successful_requests: u32,
    failed_requests: u32,
    min_latency: std::time::Duration,
    max_latency: std::time::Duration,
    avg_latency: std::time::Duration,
    total_duration: std::time::Duration,
}

impl PerformanceMetrics {
    fn new() -> Self {
        PerformanceMetrics {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            min_latency: std::time::Duration::from_secs(u64::MAX),
            max_latency: std::time::Duration::from_secs(0),
            avg_latency: std::time::Duration::from_secs(0),
            total_duration: std::time::Duration::from_secs(0),
        }
    }

    fn print_summary(&self, test_name: &str) {
        test_println!(test_name, "Performance Summary:");
        test_println!(test_name, "Total Requests: {}", self.total_requests);
        // test_println!(
        //     test_name,
        //     "Successful Requests: {}",
        //     self.successful_requests
        // );
        // test_println!(test_name, "Failed Requests: {}", self.failed_requests);
        // test_println!(
        //     test_name,
        //     "Success Rate: {:.2}%",
        //     (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        // );
        test_println!(test_name, "Min Latency: {:?}", self.min_latency);
        test_println!(test_name, "Max Latency: {:?}", self.max_latency);
        test_println!(test_name, "Avg Latency: {:?}", self.avg_latency);
        test_println!(test_name, "Total Duration: {:?}", self.total_duration);
        test_println!(
            test_name,
            "Throughput: {:.2} requests/second",
            self.total_requests as f64 / self.total_duration.as_secs_f64()
        );
    }

    fn update_latency(&mut self, latency: Duration) {
        self.min_latency = self.min_latency.min(latency);
        self.max_latency = self.max_latency.max(latency);

        let current_total = self.avg_latency.as_nanos() as u128 * (self.total_requests - 1) as u128;
        let new_avg = (current_total + latency.as_nanos()) / self.total_requests as u128;
        self.avg_latency = Duration::from_nanos(new_avg as u64);
    }
}

async fn setup_test_data(ctx: &ThroughputContext) -> Result<(), AppError> {
    let base_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let flight_numbers = vec![100, 200, 300, 400, 500];
    let capacities = vec![100, 150, 200, 250, 300];

    // Create aircraft and flight routes concurrently
    let mut setup_tasks = JoinSet::new();
    for (flight_number, capacity) in flight_numbers.into_iter().zip(capacities.into_iter()) {
        let pool = ctx.pool.clone();
        let base_date = base_date;

        setup_tasks.spawn(async move {
            // Create aircraft
            sqlx::query!(
                "INSERT INTO aircraft (aircraft_id, capacity) VALUES (?, ?)",
                flight_number,
                capacity
            )
            .execute(&pool)
            .await?;

            // Create flight route
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
                base_date,
                base_date + chrono::Duration::days(365)
            )
            .execute(&pool)
            .await?;

            Ok::<_, AppError>((flight_number, capacity))
        });
    }

    // Wait for all aircraft and routes to be created
    let mut flight_setups = Vec::new();
    while let Some(result) = setup_tasks.join_next().await {
        match result {
            Ok(Ok(setup)) => flight_setups.push(setup),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(AppError::DatabaseError(e.to_string())),
        }
    }

    // Create flights and seats concurrently
    let mut flight_tasks = JoinSet::new();
    for (flight_number, capacity) in flight_setups {
        let pool = ctx.pool.clone();
        let base_date = base_date;

        flight_tasks.spawn(async move {
            // Create flights for multiple dates concurrently
            let mut seat_tasks = JoinSet::new();

            for days in 0..30 {
                let pool = pool.clone();
                let flight_date = base_date + chrono::Duration::days(days);

                seat_tasks.spawn(async move {
                    // Create flight
                    sqlx::query!(
                        r#"
                        INSERT INTO flight (flight_number, flight_date, available_tickets, version)
                        VALUES (?, ?, ?, 1)
                        "#,
                        flight_number,
                        flight_date,
                        capacity
                    )
                    .execute(&pool)
                    .await?;

                    // Get flight_id
                    let flight_id = sqlx::query!(
                        "SELECT flight_id FROM flight WHERE flight_number = ? AND flight_date = ?",
                        flight_number,
                        flight_date
                    )
                    .fetch_one(&pool)
                    .await?
                    .flight_id;

                    // Create all seats for this flight in a single query
                    let mut query_parts = Vec::new();
                    let mut params = Vec::new();

                    for seat_number in 1..=capacity {
                        query_parts.push("(?, ?, 'AVAILABLE', 0)");
                        params.push(flight_id);
                        params.push(seat_number);
                    }

                    let query = format!(
                        r#"
                        INSERT INTO seat_info (flight_id, seat_number, seat_status, version)
                        VALUES {}
                        "#,
                        query_parts.join(",")
                    );

                    // Build the query with multiple binds
                    let mut query_builder = sqlx::query(&query);
                    for param in params {
                        query_builder = query_builder.bind(param);
                    }

                    // Execute the query
                    query_builder.execute(&pool).await?;

                    Ok::<_, AppError>(())
                });
            }

            // Wait for all flights and seats to be created+
            while let Some(result) = seat_tasks.join_next().await {
                match result {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(AppError::DatabaseError(e.to_string())),
                }
            }

            Ok::<_, AppError>(())
        });
    }

    // Wait for all flight setups to complete
    while let Some(result) = flight_tasks.join_next().await {
        match result {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(AppError::DatabaseError(e.to_string())),
        }
    }

    Ok(())
}

#[test_context(ThroughputContext)]
#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_massive_concurrent_booking(ctx: &ThroughputContext) -> Result<(), AppError> {
    let test_name = "test_concurrent_seat_booking5";
    let num_users = 500;
    let requests_per_user = 20;

    test_println!(test_name, "Setting up test data...");
    setup_test_data(ctx).await?;

    // Create users concurrently
    test_println!(test_name, "Creating {} users concurrently...", num_users);
    // Create users in batches
    const USER_BATCH_SIZE: usize = 100;
    let mut user_ids = Vec::with_capacity(num_users);

    for chunk in (0..num_users).collect::<Vec<_>>().chunks(USER_BATCH_SIZE) {
        let mut user_tasks = JoinSet::new();

        for &i in chunk {
            let user_service = ctx.user_service.clone();
            user_tasks.spawn(async move {
                let user = UserRegistrationRequest {
                    username: format!("perf_test_user_{}", i),
                    password: "test_password".to_string(),
                    role: Role::User,
                    name: format!("Performance Test User {}", i),
                    birth_date: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
                    gender: "male".to_string(),
                };
                let user_id = user_service.register_user(user).await?;
                Ok::<_, AppError>(user_id)
            });
        }

        while let Some(result) = user_tasks.join_next().await {
            match result {
                Ok(Ok(user_id)) => {
                    user_ids.push(user_id);
                    if user_ids.len() % 100 == 0 {
                        test_println!(test_name, "Created {} users so far...", user_ids.len());
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(AppError::DatabaseError(e.to_string())),
            }
        }
    }

    test_println!(test_name, "Successfully created {} users", user_ids.len());

    test_println!(test_name, "Generating booking requests...");
    let mut booking_requests = Vec::with_capacity(num_users * requests_per_user);
    let flight_numbers = vec![100, 200, 300, 400, 500];
    let num_days = 30;

    for _ in 0..num_users * requests_per_user {
        let user_id = user_ids[rand::thread_rng().gen_range(0..user_ids.len())];
        let flight_number = flight_numbers[rand::thread_rng().gen_range(0..flight_numbers.len())];
        let days = rand::thread_rng().gen_range(0..num_days);
        let flight_date =
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + chrono::Duration::days(days);

        booking_requests.push((user_id, flight_number, flight_date));
    }
    // First phase: Send first 5000 booking requests
    test_println!(test_name, "Phase 1: Sending first 5000 booking requests...");
    let metrics = PerformanceMetrics::new();
    let metrics = std::sync::Arc::new(std::sync::Mutex::new(metrics));
    let booked_tickets = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let start_time = Instant::now();

    // Now spawn all tasks
    let mut join_set = JoinSet::new();
    for &(user_id, flight_number, flight_date) in &booking_requests[..5000] {
        let ticket_service = ctx.ticket_service.clone();

        let booked_tickets = booked_tickets.clone();

        join_set.spawn(async move {
            let booking_request = TicketBookingRequest {
                flight_number: flight_number,
                flight_date: flight_date,
                preferred_seat: None,
            };

            let result = ticket_service.book_ticket(user_id, booking_request).await;

            match &result {
                Ok(_) => {
                    test_println!(
                        test_name,
                        "User {} successfully booked flight {} on {:?}",
                        user_id,
                        flight_number,
                        flight_date
                    );
                    booked_tickets
                        .lock()
                        .await
                        .push((user_id, flight_number, flight_date));
                }
                Err(e) => {
                    test_println!(
                        test_name,
                        "User {} failed to book flight {} on {:?}: {}",
                        user_id,
                        flight_number,
                        flight_date,
                        e
                    );
                }
            }
        });
    }

    // Wait for all requests to complete
    while join_set.join_next().await.is_some() {}

    // Generate seat selection requests based on successful bookings
    test_println!(test_name, "Generating seat selection requests...");
    let booked = booked_tickets.lock().await;
    let mut seat_selection_requests = Vec::with_capacity(5000);

    for _ in 0..5000 {
        if let Some(&(user_id, flight_number, flight_date)) = booked.choose(&mut rand::thread_rng())
        {
            let seat_number = rand::thread_rng().gen_range(1..=100);
            seat_selection_requests.push((user_id, flight_number, flight_date, seat_number));
        }
    }
    drop(booked);

    // Phase 2: Send remaining 5000 booking requests and 5000 seat selections
    test_println!(test_name, "Phase 2: Sending mixed requests...");
    let mut join_set = JoinSet::new();

    // Combine both types of requests into a single vector
    let mut mixed_requests = Vec::with_capacity(10000);
    for &(user_id, flight_number, flight_date) in &booking_requests[5000..] {
        mixed_requests.push(MixedRequest::Booking((user_id, flight_number, flight_date)));
    }
    for request in seat_selection_requests
        .into_iter()
        .map(MixedRequest::SeatSelection)
    {
        mixed_requests.push(request);
    }

    // Shuffle the requests
    use rand::seq::SliceRandom;
    mixed_requests.shuffle(&mut rand::thread_rng());

    
    // for request in &mixed_requests {
    //     match request {
    //         MixedRequest::Booking((user_id, flight_number, flight_date)) => {
    //             test_println!(
    //                 test_name,
    //                 "request: User book {} flight {} on {:?}",
    //                 user_id,
    //                 flight_number,
    //                 flight_date
    //             );
    //         }
    //         MixedRequest::SeatSelection((
    //             user_id,
    //             flight_number,
    //             flight_date,
    //             seat_number,
    //         )) => {
    //             test_println!(
    //                 test_name,
    //                 "request: User {} select seat on flight {} on {:?} on seat {}",
    //                 user_id,
    //                 flight_number,
    //                 flight_date,
    //                 seat_number
    //             );
    //         }
    //     }
    // }

    // Send all requests
    const BATCH_SIZE: usize = 1000;
    for chunk in mixed_requests.chunks(BATCH_SIZE) {
        for request in chunk.to_vec() {
            let ticket_service = ctx.ticket_service.clone();
            let metrics = metrics.clone();
            // let booked_tickets = booked_tickets.clone();

            join_set.spawn(async move {
                let request_start = Instant::now();

                let result = match request {
                    MixedRequest::Booking((user_id, flight_number, flight_date)) => {
                        let booking_request = TicketBookingRequest {
                            flight_number: flight_number,
                            flight_date: flight_date,
                            preferred_seat: None,
                        };

                        match ticket_service.book_ticket(user_id, booking_request).await {
                            Ok(_) => {
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    MixedRequest::SeatSelection((user_id, flight_number, flight_date, seat_number)) => {
                        match ticket_service
                            .book_seat_for_ticket(
                                user_id,
                                SeatBookingRequest {
                                    flight_number: flight_number,
                                    flight_date: flight_date,
                                    seat_number: seat_number,
                                },
                            )
                            .await
                        {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        }
                    }
                };

                let latency = request_start.elapsed();

                // Update metrics
                let mut metrics = metrics.lock().unwrap();
                metrics.total_requests += 1;
                match &result {
                    Ok(_) => {
                        metrics.successful_requests += 1;
                        match request {
                            MixedRequest::Booking((user_id, flight_number, flight_date)) => {
                                test_println!(
                                    test_name,
                                    "User {} successfully booked flight {} on {:?}",
                                    user_id,
                                    flight_number,
                                    flight_date
                                );
                            }
                            MixedRequest::SeatSelection((
                                user_id,
                                flight_number,
                                flight_date,
                                seat_number,
                            )) => {
                                test_println!(
                                    test_name,
                                    "User {} successfully selected seat on flight {} on {:?} on seat {}",
                                    user_id,
                                    flight_number,
                                    flight_date,
                                    seat_number
                                );
                            }
                        }
                    }
                    Err(e) => {
                        metrics.failed_requests += 1;
                        match request {
                            MixedRequest::Booking((user_id, flight_number, flight_date)) => {
                                test_println!(
                                    test_name,
                                    "User {} failed to booked flight {} on {:?}: {}",
                                    user_id,
                                    flight_number,
                                    flight_date, 
                                    e
                                );
                            }
                            MixedRequest::SeatSelection((
                                user_id,
                                flight_number,
                                flight_date,
                                seat_number,
                            )) => {
                                test_println!(
                                    test_name,
                                    "User {} failed to select seat on flight {} on {:?} on seat {}: {}",
                                    user_id,
                                    flight_number,
                                    flight_date,
                                    seat_number, 
                                    e
                                );
                            }
                        }
                    },
                }
                metrics.update_latency(latency);
            });
            
            while join_set.join_next().await.is_some() {}
        }
    }

    while join_set.join_next().await.is_some() {}

    // Update total duration and print results
    let mut metrics = metrics.lock().unwrap();
    metrics.total_duration = start_time.elapsed();
    metrics.print_summary(test_name);

    Ok(())
}
