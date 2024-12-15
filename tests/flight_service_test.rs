use airline_booking_system::{
    models::flight::FlightSearchQuery,
    services::flight_service::FlightService,
    utils::error::AppError,
};
use async_trait::async_trait;
use chrono::{NaiveDate, NaiveTime};
use ctor::dtor;
use sqlx::mysql::MySqlPool as Pool;
use test_context::{test_context, AsyncTestContext};

mod common {
    pub mod test_utils;
}
use common::test_utils::TestDb;

struct FlightServiceContext {
    pool: Pool,
    flight_service: FlightService,
}

#[dtor]
fn cleanup() {
    if let Err(e) = TestDb::cleanup_database_sync() {
        eprintln!("Failed to cleanup test database: {}", e);
    }
}

#[async_trait]
impl AsyncTestContext for FlightServiceContext {
    async fn setup() -> Self {
        let pool = TestDb::get_instance(file!())
            .await
            .expect("Failed to get test database instance");

        let flight_service = FlightService::new(pool.clone());

        FlightServiceContext { pool, flight_service }
    }

    async fn teardown(self) {
        let _ = sqlx::query("SELECT 1").execute(&self.pool).await;
    }
}

impl FlightServiceContext {
    // Helper method to create test flight data
    async fn create_test_flight(
        &self,
        flight_number: i32,
        departure_city: &str,
        destination_city: &str,
        flight_date: NaiveDate,
        available_tickets: i32,
    ) -> Result<(), AppError> {
        // Create aircraft for test
        let aircraft_id = 999; 
        let capacity = 10;    
        sqlx::query!(
            r#"
            INSERT INTO aircraft (aircraft_id, capacity)
            VALUES (?, ?)
            ON DUPLICATE KEY UPDATE capacity = ?
            "#,
            aircraft_id,
            capacity,
            capacity
        )
        .execute(&self.pool)
        .await?;

        // Create flight route
        sqlx::query!(
            r#"
            INSERT INTO flight_route (
                flight_number, 
                departure_city, 
                destination_city, 
                departure_time, 
                arrival_time,
                aircraft_id,
                start_date
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            flight_number,
            departure_city,
            destination_city,
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            aircraft_id,
            flight_date
        )
        .execute(&self.pool)
        .await?;

        // Create flight
        let flight_result = sqlx::query!(
            r#"
            INSERT INTO flight (flight_number, flight_date, available_tickets)
            VALUES (?, ?, ?)
            "#,
            flight_number,
            flight_date,
            available_tickets
        )
        .execute(&self.pool)
        .await?;

        // Get the new created flight_id
        let flight_id = flight_result.last_insert_id() as i32;

        // Create seat_info records for each seat
        for seat_number in 1..=capacity {
            sqlx::query!(
                r#"
                INSERT INTO seat_info (flight_id, seat_number, seat_status)
                VALUES (?, ?, 'AVAILABLE')
                "#,
                flight_id,
                seat_number
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}

#[test_context(FlightServiceContext)]
#[tokio::test]
async fn test_search_flights_single_date(ctx: &FlightServiceContext) -> Result<(), AppError> {
    // Create test flight(s)
    let departure_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    ctx.create_test_flight(101, "Beijing", "Shanghai", departure_date, 100)
        .await?;
    ctx.create_test_flight(102, "Beijing", "Shanghai", departure_date, 0)
        .await?;
    ctx.create_test_flight(103, "Shanghai", "Beijing", departure_date, 100)
        .await?;

    // execute search
    let search_query = FlightSearchQuery {
        departure_city: "Beijing".to_string(),
        destination_city: "Shanghai".to_string(),
        departure_date,
        end_date: None,
    };

    let result = ctx.flight_service.search_flights(search_query).await?;

    // Assert
    assert_eq!(result.flights.len(), 1);
    assert_eq!(result.flights[0].flight_number, 101);
    assert_eq!(result.flights[0].departure_city, "Beijing");
    assert_eq!(result.flights[0].destination_city, "Shanghai");
    assert_eq!(result.flights[0].available_tickets, 100);

    Ok(())
}

#[test_context(FlightServiceContext)]
#[tokio::test]
async fn test_search_flights_date_range(ctx: &FlightServiceContext) -> Result<(), AppError> {
    // Create test flight(s)
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let middle_date = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end_date = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
    ctx.create_test_flight(201, "Toronto", "Ottawa", start_date, 100)
        .await?;
    ctx.create_test_flight(202, "Toronto", "Ottawa", middle_date, 100)
        .await?;
    ctx.create_test_flight(203, "Toronto", "Ottawa", end_date, 100)
        .await?;
    ctx.create_test_flight(204, "Ottawa", "Toronto", middle_date, 100)
        .await?;

    // execute search
    let search_query = FlightSearchQuery {
        departure_city: "Toronto".to_string(),
        destination_city: "Ottawa".to_string(),
        departure_date: start_date,
        end_date: Some(end_date),
    };

    let result = ctx.flight_service.search_flights(search_query).await?;

    // Assert
    assert_eq!(result.flights.len(), 3);

    for flight in result.flights {
        assert_eq!(flight.departure_city, "Toronto");
        assert_eq!(flight.destination_city, "Ottawa");
        assert!(flight.flight_date >= start_date && flight.flight_date <= end_date);
        assert!(flight.available_tickets > 0);
    }

    Ok(())
}

#[test_context(FlightServiceContext)]
#[tokio::test]
async fn test_get_available_seats(ctx: &FlightServiceContext) -> Result<(), AppError> {
    let flight_number = 301;
    let flight_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    ctx.create_test_flight(
        flight_number,
        "New York",
        "Los Angeles",
        flight_date,
        10
    ).await?;

    // Execute query
    let result = ctx.flight_service
        .get_available_seats(flight_number, flight_date)
        .await?;

    // Assert
    assert_eq!(result.available_seats.len(), 10); // There should be 10 seats available 
    
    for i in 1..=10 {
        assert!(result.available_seats.contains(&i));
    }

    Ok(())
}

#[test_context(FlightServiceContext)]
#[tokio::test]
async fn test_get_available_seats_nonexistent_flight(ctx: &FlightServiceContext) -> Result<(), AppError> {
    let flight_number = 302;
    let flight_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let result = ctx.flight_service
        .get_available_seats(flight_number, flight_date)
        .await;

    // Assert
    match result {
        Err(AppError::NotFound(msg)) => {
            assert_eq!(msg, "Flight not found");
            Ok(())
        }
        _ => panic!("Expected NotFound error for non-existent flight"),
    }
}

#[test_context(FlightServiceContext)]
#[tokio::test]
async fn test_get_available_seats_with_mixed_status(ctx: &FlightServiceContext) -> Result<(), AppError> {
    // Create test flight
    let flight_number = 303;
    let flight_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    ctx.create_test_flight(
        flight_number,
        "New York",
        "Los Angeles",
        flight_date,
        10
    ).await?;

    // Get flight_id for updating seat status
    let flight = sqlx::query!(
        r#"
        SELECT flight_id 
        FROM flight 
        WHERE flight_number = ? AND flight_date = ?
        "#,
        flight_number,
        flight_date
    )
    .fetch_one(&ctx.pool)
    .await?;

    // Set some seats to BOOKED and UNAVAILABLE
    sqlx::query!(
        r#"
        UPDATE seat_info 
        SET seat_status = 'BOOKED'
        WHERE flight_id = ? AND seat_number IN (2, 4, 6)
        "#,
        flight.flight_id
    )
    .execute(&ctx.pool)
    .await?;

    sqlx::query!(
        r#"
        UPDATE seat_info 
        SET seat_status = 'UNAVAILABLE'
        WHERE flight_id = ? AND seat_number IN (8, 10)
        "#,
        flight.flight_id
    )
    .execute(&ctx.pool)
    .await?;

    // Execute query
    let result = ctx.flight_service
        .get_available_seats(flight_number, flight_date)
        .await?;

    // Assert
    assert_eq!(result.available_seats.len(), 5); // There should be 5 seats available 
    
    let expected_available_seats = vec![1, 3, 5, 7, 9];
    for seat_number in expected_available_seats {
        assert!(
            result.available_seats.contains(&seat_number),
            "Seat {} should be available", 
            seat_number
        );
    }

    let unavailable_seats = vec![2, 4, 6, 8, 10];
    for seat_number in unavailable_seats {
        assert!(
            !result.available_seats.contains(&seat_number),
            "Seat {} should not be available",
            seat_number
        );
    }

    Ok(())
}