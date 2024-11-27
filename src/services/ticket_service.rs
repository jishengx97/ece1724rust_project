use crate::models::flight::Flight;
use crate::models::ticket::{
    BookingHistoryDetail, BookingHistoryResponse, TicketBookingRequest, TicketBookingResponse,
};
use crate::services::flight_service::FlightService;
use crate::utils::error::{AppError, AppResult};
use chrono::{NaiveDate, NaiveTime};
use sqlx::mysql::MySqlQueryResult;
use sqlx::MySqlPool;

pub struct TicketService {
    pool: MySqlPool,
    flight_service: FlightService,
}

impl TicketService {
    pub fn new(pool: MySqlPool) -> Self {
        TicketService {
            flight_service: FlightService::new(pool.clone()),
            pool,
        }
    }

    pub async fn book_ticket(
        &self,
        user_id: i32,
        request: TicketBookingRequest,
    ) -> AppResult<TicketBookingResponse> {
        let mut tx = self.pool.begin().await?;

        // get the flight information
        // TODO: it is assumed legal here
        let flight = sqlx::query_as!(
            Flight,
            r#"SELECT flight_id, flight_number, flight_date, available_tickets, version 
            FROM flight WHERE flight_number = ? AND flight_date = ? FOR UPDATE"#,
            request.flight_number,
            request.flight_date
        )
        .fetch_one(&mut *tx)
        .await?;

        println!("Searched flight {}!", flight.flight_id);

        // TODO: check the legality of the preferred seat, even if it's a provided value
        // check if the preferred seat is available
        if let Some(seat) = request.preferred_seat {
            if !self
                .flight_service
                .is_seat_available(request.flight_number, seat)
                .await?
            {
                return Err(AppError::Conflict("Seat is not available".into()));
            }
        }

        // everything is legal, create the ticket
        let result: MySqlQueryResult;
        if let Some(seat) = request.preferred_seat {
            result = sqlx::query!(
                r#"
                INSERT INTO ticket (customer_id, flight_id, seat_number, flight_date, flight_number)
                VALUES (?, ?, ?, ?, ?)
                "#,
                user_id,
                flight.flight_id,
                seat,
                flight.flight_date,
                flight.flight_number
            )
            .execute(&mut *tx)
            .await?;
        } else {
            result = sqlx::query!(
                r#"
                INSERT INTO ticket (customer_id, flight_id, flight_date, flight_number)
                VALUES (?, ?, ?, ?)
                "#,
                user_id,
                flight.flight_id,
                flight.flight_date,
                flight.flight_number
            )
            .execute(&mut *tx)
            .await?;
        }

        let ticket_id = result.last_insert_id() as i32;

        println!("inserted {}", ticket_id);

        tx.commit().await?;

        Ok(TicketBookingResponse {
            ticket_id,
            flight_details: format!("Flight {} on {}", flight.flight_number, flight.flight_date),
            seat_number: request.preferred_seat,
            booking_status: "Confirmed".to_string(),
        })
    }

    pub async fn get_history(&self, user_id: i32) -> AppResult<BookingHistoryResponse> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                f.flight_number, 
                t.seat_number,
                fr.departure_city, 
                fr.destination_city, 
                f.flight_date,
                fr.departure_time, 
                fr.arrival_time
            FROM ticket t
            INNER JOIN flight f ON t.flight_id = f.flight_id
            INNER JOIN flight_route fr ON f.flight_number = fr.flight_number
            WHERE t.customer_id = ?
            ORDER BY f.flight_date DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let flights: Vec<BookingHistoryDetail> = rows
            .iter()
            .map(|row| BookingHistoryDetail {
                flight_number: row.flight_number,
                seat_number: if let Some(s) = row.seat_number {
                    s.to_string()
                } else {
                    String::from("Not Selected")
                },
                departure_city: row.departure_city.clone(),
                destination_city: row.destination_city.clone(),
                flight_date: NaiveDate::from_ymd_opt(
                    row.flight_date.year() as i32,
                    row.flight_date.month() as u32,
                    row.flight_date.day() as u32,
                )
                .unwrap(),
                departure_time: NaiveTime::from_hms_opt(
                    row.departure_time.hour() as u32,
                    row.departure_time.minute() as u32,
                    row.departure_time.second() as u32,
                )
                .unwrap(),
                arrival_time: NaiveTime::from_hms_opt(
                    row.arrival_time.hour() as u32,
                    row.arrival_time.minute() as u32,
                    row.arrival_time.second() as u32,
                )
                .unwrap(),
            })
            .collect();

        // Build the response
        Ok(BookingHistoryResponse { flights })
    }
}
