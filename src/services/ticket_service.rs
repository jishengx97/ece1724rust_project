use crate::models::flight::Flight;
use crate::models::ticket::{TicketBookingRequest, TicketBookingResponse};
use crate::services::flight_service::FlightService;
use crate::utils::error::{AppError, AppResult};
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
}
