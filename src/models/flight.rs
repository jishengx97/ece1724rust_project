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
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Flight {
    pub flight_id: i32,
    pub flight_number: i32,
    pub flight_date: NaiveDate,
    pub available_tickets: i32,
    pub version: Option<i32>,  // 用于乐观锁
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FlightSearchRequest {
    pub departure_city: String,
    pub destination_city: String,
    #[serde(flatten)]
    pub date_criteria: FlightDateCriteria,
}

// Support single date or date range
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FlightDateCriteria {
    Single {
        departure_date: NaiveDate,
    },
    Range {
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
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