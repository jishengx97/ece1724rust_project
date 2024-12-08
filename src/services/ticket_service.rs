use crate::models::flight::Flight;
use crate::models::flight::SeatStatus;
use crate::models::ticket::{
    BookingHistoryDetail, BookingHistoryResponse, SeatBookingRequest, TicketBookingRequest,
    TicketBookingResponse,
};
use crate::utils::error::{AppError, AppResult};
use chrono::{NaiveDate, NaiveTime};
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct TicketService {
    pool: MySqlPool,
}

impl TicketService {
    pub fn new(pool: MySqlPool) -> Self {
        TicketService { pool }
    }

    pub async fn book_ticket(
        &self,
        user_id: i32,
        request: TicketBookingRequest,
    ) -> AppResult<TicketBookingResponse> {
        // get the flight information
        // TODO: it is assumed legal here

        // TODO: do not allow re-booking the same flight for now
        let existing_ticket = sqlx::query!(
            r#"SELECT id, seat_number FROM ticket 
            WHERE customer_id = ? 
            AND flight_number = ?
            AND flight_date = ?"#,
            user_id,
            request.flight_number,
            request.flight_date
        )
        .fetch_optional(&self.pool)
        .await?;

        match existing_ticket {
            Some(_) => {
                return Err(AppError::BadRequest(
                    "Cannot re-book the same flight".to_string(),
                ))
            }
            None => {}
        };

        let mut retries = 0;
        let max_retries = 3;

        let mut flight: Flight;

        loop {
            flight = sqlx::query_as!(
                Flight,
                r#"
                SELECT flight_id, flight_number, flight_date, available_tickets, version 
                FROM flight 
                WHERE flight_number = ? 
                AND flight_date = ?
                "#,
                request.flight_number,
                request.flight_date
            )
            .fetch_one(&self.pool)
            .await?;

            println!("Searched flight {}!", flight.flight_id);

            if flight.available_tickets == 0 {
                return Err(AppError::ValidationError(
                    "This flight is fully booked.".to_string(),
                ));
            }

            // Create a ticket for the user first, and worry about the seat later.
            // We book a ticket for the user regardless of whether the preferred seat is available
            let mut tx = self.pool.begin().await?;

            let update_result = sqlx::query!(
                r#"
                UPDATE flight
                set available_tickets = available_tickets - 1, 
                    version = version + 1
                where flight_id = ?
                AND version = ?
                "#,
                flight.flight_id,
                flight.version,
            )
            .execute(&mut *tx)
            .await?;

            if update_result.rows_affected() == 0 {
                tx.rollback().await?;
                retries += 1;

                if retries > max_retries {
                    break;
                }
            } else {
                tx.commit().await?;
                break;
            }
        }

        if retries > max_retries {
            // We did not success
            return Err(AppError::ValidationError(
                "We failed to book your flight at this time, please retry.".to_string(),
            ));
        }

        let result = sqlx::query!(
            r#"
            INSERT INTO ticket (customer_id, flight_id, flight_date, flight_number)
            VALUES (?, ?, ?, ?)
            "#,
            user_id,
            flight.flight_id,
            flight.flight_date,
            flight.flight_number
        )
        .execute(&self.pool)
        .await?;

        let ticket_id = result.last_insert_id() as i32;
        println!("inserted {}", ticket_id);

        let response = TicketBookingResponse {
            ticket_id,
            flight_details: format!("Flight {} on {}", flight.flight_number, flight.flight_date),
            seat_number: None,
            booking_status: "Confirmed.".to_string(),
        };

        // Successfully booked a ticket, now do the seat part.
        match request.preferred_seat {
            Some(prefered_seat) => {
                let book_seat_result = self
                    .book_seat_for_ticket(
                        user_id,
                        SeatBookingRequest {
                            flight_number: flight.flight_number.to_string(),
                            flight_date: NaiveDate::from_ymd_opt(
                                flight.flight_date.year() as i32,
                                flight.flight_date.month() as u32,
                                flight.flight_date.day() as u32,
                            )
                            .unwrap(),
                            seat_number: prefered_seat,
                        },
                    )
                    .await;
                match book_seat_result {
                    Ok(_) => return Ok(TicketBookingResponse {
                        seat_number: Some(prefered_seat),
                        ..response
                    }),
                    Err(_) => return Ok(TicketBookingResponse {
                        booking_status: "Confirmed booking, however the preferred seat is currently unavaiable, please try again later.".to_string(),
                        ..response
                    }),
                }
            }
            None => return Ok(response),
        }
    }

    pub async fn book_seat(
        &self,
        customer_id: i32,
        flight_id: i32,
        new_seat_number: i32,
        old_seat_number: Option<i32>,
    ) -> AppResult<bool> {
        let mut retries = 0;
        let max_retries = 3;

        while retries < max_retries {
            let mut tx = self.pool.begin().await?;

            // get the new seat information
            let new_seat_info = sqlx::query!(
                r#"
                SELECT seat_status as "seat_status: SeatStatus", version
                FROM seat_info
                WHERE flight_id = ? AND seat_number = ?
                "#,
                flight_id,
                new_seat_number
            )
            .fetch_optional(&mut *tx)
            .await?;

            let new_seat_info = match new_seat_info {
                Some(info) => info,
                None => {
                    return Err(AppError::ValidationError(
                        "The new seat is not found".to_string(),
                    ))
                }
            };

            if new_seat_info.seat_status != SeatStatus::Available {
                return Err(AppError::ValidationError(
                    "The new seat is already booked or unavailable".to_string(),
                ));
            }

            // update the new seat information
            let update_result = sqlx::query!(
                r#"
                UPDATE seat_info
                SET seat_status = ?,
                    version = version + 1
                WHERE flight_id = ? 
                AND seat_number = ? 
                AND version = ?
                AND seat_status = 'AVAILABLE'
                "#,
                SeatStatus::Booked.to_string(),
                flight_id,
                new_seat_number,
                new_seat_info.version
            )
            .execute(&mut *tx)
            .await?;

            if update_result.rows_affected() == 0 {
                tx.rollback().await?;
                retries += 1;
                continue;
            }

            // update the old seat information
            if let Some(old_seat) = old_seat_number {
                sqlx::query!(
                    r#"
                    UPDATE seat_info
                    SET seat_status = 'AVAILABLE',
                        version = version + 1
                    WHERE flight_id = ? AND seat_number = ?
                    "#,
                    flight_id,
                    old_seat
                )
                .execute(&mut *tx)
                .await?;
            }

            // update the ticket information
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

            tx.commit().await?;
            return Ok(true);
        }

        Err(AppError::Conflict(
            "Failed to book seat after maximum retries".into(),
        ))
    }

    pub async fn book_seat_for_ticket(
        &self,
        customer_id: i32,
        request: SeatBookingRequest,
    ) -> AppResult<bool> {
        let flight = sqlx::query!(
            r#"SELECT flight_id FROM flight 
            WHERE flight_number = ? AND flight_date = ?"#,
            request.flight_number,
            request.flight_date
        )
        .fetch_one(&self.pool)
        .await?;

        let ticket = sqlx::query!(
            r#"SELECT id, seat_number FROM ticket 
            WHERE customer_id = ? AND flight_id = ?"#,
            customer_id,
            flight.flight_id
        )
        .fetch_optional(&self.pool)
        .await?;

        println!("ticket: {:?}", ticket);

        let ticket = match ticket {
            Some(t) => t,
            None => {
                return Err(AppError::BadRequest(
                    "Customer does not have a ticket for this flight".into(),
                ))
            }
        };

        if let Some(current_seat) = ticket.seat_number {
            if current_seat == request.seat_number {
                return Err(AppError::BadRequest(
                    "Cannot book the same seat you already have".into(),
                ));
            }
        }

        // book the seat
        self.book_seat(
            customer_id,
            flight.flight_id,
            request.seat_number,
            ticket.seat_number,
        )
        .await
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
