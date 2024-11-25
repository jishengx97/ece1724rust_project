use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, sqlx::FromRow)]
pub struct FlightRoute {
    pub flight_number: i32,
    pub departure_city: String,
    pub destination_city: String,
    pub departure_time: NaiveTime,
    pub arrival_time: NaiveTime,
    pub aircraft_id: i32,
    pub overbooking: Decimal,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>
}

#[derive(Debug, sqlx::FromRow)]
pub struct Flight {
    pub flight_id: i32,
    pub flight_number: i32,
    pub flight_date: NaiveDate,
    pub available_tickets: i32,
    pub version: Option<i32>
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlightSearchQuery {
    pub departure_city: String,
    pub destination_city: String,
    pub departure_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FlightSearchResponse {
    pub flights: Vec<FlightDetail>,
}

// Single Flight Detail in FlightSearchResponse
#[derive(Debug, Serialize, JsonSchema)]
pub struct FlightDetail {
    pub flight_id: i32,
    pub flight_number: i32,
    pub departure_city: String,
    pub destination_city: String,
    pub departure_time: NaiveTime,
    pub arrival_time: NaiveTime,
    pub available_tickets: i32,
    pub flight_date: NaiveDate,
}

// Seat Status Enum
#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "ENUM")]
pub enum SeatStatus {
    #[sqlx(rename = "AVAILABLE")]
    Available,
    #[sqlx(rename = "UNAVAILABLE")]
    Unavailable,
    #[sqlx(rename = "BOOKED")]
    Booked,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AvailableSeatsResponse {
    pub available_seats: Vec<i32>
}