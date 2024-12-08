use chrono::{NaiveDate, NaiveTime};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct Ticket {
    pub id: i32,
    pub customer_id: i32,
    pub flight_id: i32,
    pub seat_number: Option<i32>,
    pub flight_date: NaiveDate,
    pub flight_number: i32,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct TicketBookingRequest {
    pub flight_number: i32,
    pub flight_date: NaiveDate,
    pub preferred_seat: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TicketBookingResponse {
    pub ticket_id: i32,
    pub flight_details: String,
    pub seat_number: Option<i32>,
    pub booking_status: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SeatBookingRequest {
    pub flight_number: String,
    pub flight_date: chrono::NaiveDate,
    pub seat_number: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BookingHistoryDetail {
    pub flight_number: i32,
    pub seat_number: String,
    pub departure_city: String,
    pub destination_city: String,
    pub flight_date: NaiveDate,
    pub departure_time: NaiveTime,
    pub arrival_time: NaiveTime,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct BookingHistoryResponse {
    pub flights: Vec<BookingHistoryDetail>,
}
