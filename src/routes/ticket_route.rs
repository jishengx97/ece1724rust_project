use crate::models::ticket::{BookingHistoryResponse, TicketBookingRequest, SeatBookingRequest};
use crate::services::ticket_service::TicketService;
use crate::utils::error::AppError;
use crate::utils::jwt::AuthenticatedUser;
use rocket::serde::json::Json;
use rocket::serde::json::{json, Value};
use rocket::State;
use rocket_okapi::openapi;

#[openapi(tag = "Book")]
#[post("/tickets/book", format = "json", data = "<request>")]
pub async fn book_ticket(
    request: Json<TicketBookingRequest>,
    auth: AuthenticatedUser,
    ticket_service: &State<TicketService>,
) -> Result<Json<Value>, AppError> {
    let response = ticket_service
        .book_ticket(auth.user_id, request.into_inner())
        .await?;

    Ok(Json(json!(response)))
}

#[openapi(tag = "Book")]
#[post("/tickets/seat/book", format = "json", data = "<request>")]
pub async fn book_seat_for_ticket(
    request: Json<SeatBookingRequest>,
    auth: AuthenticatedUser,
    ticket_service: &State<TicketService>,
) -> Result<Json<Value>, AppError> {
    let success = ticket_service
        .book_seat_for_ticket(
            auth.user_id,
            request.into_inner()
        )
        .await?;

    Ok(Json(json!({ "success": success })))
}

#[openapi(tag = "Book")]
#[get("/history")]
pub async fn get_history(
    _auth: AuthenticatedUser,
    ticket_service: &State<TicketService>,
) -> Result<Json<BookingHistoryResponse>, AppError> {
    let response = ticket_service.get_history(_auth.user_id).await?;
    Ok(Json(response))
}
