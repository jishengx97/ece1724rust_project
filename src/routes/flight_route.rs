use chrono::NaiveDate;
use rocket::serde::json::Json;
use rocket::State;
use rocket::serde::json::{json, Value}; 
use rocket_okapi::openapi; 
use crate::models::flight::{FlightSearchQuery, FlightSearchResponse, AvailableSeatsResponse};
use crate::services::flight_service::FlightService;
use crate::utils::error::AppError;
use crate::utils::jwt::AuthenticatedUser;

/// Search flights
#[openapi(tag = "Flights")]
#[get("/flights/search?<departure_city>&<destination_city>&<departure_date>&<end_date>")]
pub async fn search_flights(
    departure_city: String,
    destination_city: String,
    departure_date: String,
    end_date: Option<String>,
    _auth: AuthenticatedUser,
    flight_service: &State<FlightService>,
) -> Result<Json<FlightSearchResponse>, AppError> {
    let departure_date = NaiveDate::parse_from_str(&departure_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid departure date format".into()))?;
    
    let end_date = 
    if let Some(date) = end_date {
        Some(NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid end date format".into()))?)
    } else {
        None
    };
    
    let query = FlightSearchQuery {
        departure_city,
        destination_city,
        departure_date,
        end_date,
    };
    let flights = flight_service.search_flights(query).await?;
    Ok(Json(flights))
}

/// Get available seats for a flight
#[openapi(tag = "Flights")]
#[get("/flights/availableSeats?<flight_number>&<flight_date>")]
pub async fn get_available_seats(
    flight_number: i32,
    flight_date: String,
    _auth: AuthenticatedUser,
    flight_service: &State<FlightService>,
) -> Result<Json<AvailableSeatsResponse>, AppError> {
    let flight_date = NaiveDate::parse_from_str(&flight_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid flight date format".into()))?;
    
    let available_seats = flight_service.get_available_seats(flight_number, flight_date).await?;
    Ok(Json(available_seats))
}