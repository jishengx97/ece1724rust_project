use crate::models::flight::{
    AvailableSeatsResponse, FlightDetail, FlightSearchQuery, FlightSearchResponse,
};
use crate::utils::error::AppError;
use crate::utils::error::AppResult;
use sqlx::types::chrono::{NaiveDate, NaiveTime};
use sqlx::MySqlPool;

pub struct FlightService {
    pool: MySqlPool,
}

impl FlightService {
    pub fn new(pool: MySqlPool) -> Self {
        FlightService { pool }
    }

    // Search available flights
    pub async fn search_flights(
        &self,
        search_query: FlightSearchQuery,
    ) -> AppResult<FlightSearchResponse> {
        let flights = match search_query.end_date {
            // If end date is provided, search by date range
            Some(end_date) => {
                // Search by date range
                sqlx::query_as!(
                    FlightDetail,
                    r#"
                    SELECT 
                        f.flight_id,
                        f.flight_number,
                        fr.departure_city,
                        fr.destination_city,
                        fr.departure_time as "departure_time: NaiveTime",
                        fr.arrival_time as "arrival_time: NaiveTime",
                        f.available_tickets,
                        f.flight_date as "flight_date: NaiveDate"
                    FROM flight f
                    JOIN flight_route fr ON f.flight_number = fr.flight_number
                    WHERE fr.departure_city = ?
                    AND fr.destination_city = ?
                    AND f.flight_date BETWEEN ? AND ?
                    AND f.available_tickets > 0
                    "#,
                    search_query.departure_city,
                    search_query.destination_city,
                    search_query.departure_date,
                    end_date
                )
                .fetch_all(&self.pool)
                .await?
            }
            // If end date is not provided, search by single date
            None => {
                sqlx::query_as!(
                    FlightDetail,
                    r#"
                    SELECT 
                        f.flight_id,
                        f.flight_number,
                        fr.departure_city,
                        fr.destination_city,
                        fr.departure_time as "departure_time: NaiveTime",
                        fr.arrival_time as "arrival_time: NaiveTime",
                        f.available_tickets,
                        f.flight_date as "flight_date: NaiveDate"
                    FROM flight f
                    JOIN flight_route fr ON f.flight_number = fr.flight_number
                    WHERE fr.departure_city = ?
                    AND fr.destination_city = ?
                    AND f.flight_date = ?
                    AND f.available_tickets > 0
                    "#,
                    search_query.departure_city,
                    search_query.destination_city,
                    search_query.departure_date
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(FlightSearchResponse { flights })
    }

    pub async fn get_available_seats(
        &self,
        flight_number: i32,
        flight_date: NaiveDate,
    ) -> AppResult<AvailableSeatsResponse> {
        // Get flight id by flight number and flight date
        let flight = sqlx::query!(
            r#"
            SELECT flight_id 
            FROM flight 
            WHERE flight_number = ? AND flight_date = ?
            "#,
            flight_number,
            flight_date
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Flight not found".into()))?;

        // Get all available seats
        let available_seats = sqlx::query!(
            r#"
            SELECT seat_number
            FROM seat_info
            WHERE flight_id = ? AND seat_status = 'AVAILABLE'
            "#,
            flight.flight_id
        )
        .fetch_all(&self.pool)
        .await?;

        // Convert query result to Vec<i32>
        let available_seats: Vec<i32> = available_seats
            .into_iter()
            .map(|row| row.seat_number)
            .collect();

        Ok(AvailableSeatsResponse { available_seats })
    }

    // assume seat is available for now
    pub async fn is_seat_available(&self, _flight_id: i32, _seat_number: i32) -> AppResult<bool> {
        Ok(true)
    }
}
