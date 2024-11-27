use crate::models::flight::Flight;
use crate::models::ticket::{
    BookingHistoryDetail, BookingHistoryResponse, TicketBookingRequest, TicketBookingResponse, SeatBookingRequest
};
use crate::services::flight_service::FlightService;
use crate::utils::error::{AppError, AppResult};
use chrono::{NaiveDate, NaiveTime};
use sqlx::mysql::MySqlQueryResult;
use sqlx::MySqlPool;
use crate::models::flight::SeatStatus;

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

        // Todo: Update the available_tickets in flight table
        // TODO: Update the seat_status in seat_info table
        Ok(TicketBookingResponse {
            ticket_id,
            flight_details: format!("Flight {} on {}", flight.flight_number, flight.flight_date),
            seat_number: request.preferred_seat,
            booking_status: "Confirmed".to_string(),
        })
    }

    pub async fn book_seat(
        &self,
        customer_id: i32,
        flight_id: i32,
        new_seat_number: i32,
        old_seat_number: Option<i32>
    ) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;
    
        // println!("Start to book Seat for customer: {}, flight_id: {}, new_seat_number: {}, old_seat_number: {:?}", customer_id, flight_id, new_seat_number, old_seat_number);
        let new_seat_status = sqlx::query!(
            r#"
            SELECT seat_status as "seat_status: SeatStatus"
            FROM seat_info
            WHERE flight_id = ? AND seat_number = ?
            FOR UPDATE
            "#,
            flight_id,
            new_seat_number
        )
        .fetch_optional(&mut *tx)
        .await
        .inspect_err(|e| println!("Get new seat status failed: {:?}", e))?;
    
        //println!("new_seat_status: {:?}", new_seat_status);
        let new_seat_status = match new_seat_status {
            Some(status) => status.seat_status,
            None => return Err(AppError::ValidationError("The new seat is not found".to_string())),
        };
    
        if new_seat_status != SeatStatus::Available {
            return Err(AppError::ValidationError("The new seat is already booked or unavailable".to_string()));
        }

        //println!("old_seat_number: {:?}", old_seat_number);
        if let Some(old_seat) = old_seat_number {
            sqlx::query!(
                r#"
                UPDATE seat_info
                SET seat_status = 'AVAILABLE'
                WHERE flight_id = ? AND seat_number = ?
                "#,
                // SeatStatus::Available.to_string(),
                flight_id,
                old_seat
            )
            .execute(&mut *tx)
            .await?;
        }

        //println!("Updated the old seat to available");
        sqlx::query!(
            r#"
            UPDATE seat_info
            SET seat_status = ?
            WHERE flight_id = ? AND seat_number = ?
            "#,
            SeatStatus::Booked.to_string(),
            flight_id,
            new_seat_number
        )
        .execute(&mut *tx)
        .await?;

        // println!("Updated the new seat to booked");

        sqlx::query!(
            r#"
            UPDATE ticket
            SET seat_number = ?
            WHERE customer_id = ? AND flight_id = ?
            "#,
            new_seat_number,
            customer_id,
            flight_id
        )
        .execute(&mut *tx)
        .await?;

        // println!("Updated the ticket to new seat");
        tx.commit().await?;
        Ok(true)
    }
    
    pub async fn book_seat_for_ticket(
        &self,
        customer_id: i32,
        request: SeatBookingRequest,
    ) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;

        let flight = sqlx::query!(
            r#"SELECT flight_id FROM flight 
            WHERE flight_number = ? AND flight_date = ?"#,
            request.flight_number,
            request.flight_date
        )
        .fetch_one(&mut *tx)
        .await?;

        let ticket = sqlx::query!(
            r#"SELECT id, seat_number FROM ticket 
            WHERE customer_id = ? AND flight_id = ?"#,
            customer_id,
            flight.flight_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        println!("ticket: {:?}", ticket);

        let ticket = match ticket {
            Some(t) => t,
            None => return Err(AppError::BadRequest("Customer does not have a ticket for this flight".into())),
        };

        // book the seat
        self.book_seat(
            customer_id,
            flight.flight_id,
            request.seat_number,
            ticket.seat_number
        ).await
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

