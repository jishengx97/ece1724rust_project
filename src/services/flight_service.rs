use crate::models::flight::{FlightSearchQuery, FlightSearchResponse, FlightDetail};
use crate::utils::error::AppResult;
use sqlx::MySqlPool;
use sqlx::types::chrono::{NaiveDate, NaiveTime};

pub struct FlightService {
    pool: MySqlPool,
}

impl FlightService {
    pub fn new(pool: MySqlPool) -> Self {
        FlightService { pool }
    }

    // Search available flights
    pub async fn search_flights(&self, query: FlightSearchQuery) -> AppResult<FlightSearchResponse> {
        let flights = match query.end_date {
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
                    query.departure_city,
                    query.destination_city,
                    query.departure_date,
                    end_date
                )
                .fetch_all(&self.pool)
                .await?
            },
            None => {
                // Search by single date
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
                    query.departure_city,
                    query.destination_city,
                    query.departure_date
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(FlightSearchResponse { flights })
    }
}
