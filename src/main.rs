#[macro_use]
extern crate rocket;
extern crate rocket_okapi;

mod models;
mod routes;
mod services;
mod swagger;
mod utils;

use crate::swagger::swagger_ui;
use dotenv::dotenv;
use rocket::fairing::AdHoc;
use rocket_okapi::openapi_get_routes;
use rocket_okapi::swagger_ui::*;
use sqlx::MySqlPool;

#[launch]
async fn rocket() -> _ {
    dotenv().ok();

    // Connect to the database
    let pool =
        MySqlPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .await
            .expect("Failed to connect to database");

    // Initialize the user service
    let user_service = services::user_service::UserService::new(pool.clone());
    let flight_service = services::flight_service::FlightService::new(pool.clone());
    let ticket_service = services::ticket_service::TicketService::new(pool.clone());

    rocket::build()
        .manage(user_service)
        .manage(flight_service)
        .manage(ticket_service)
        .mount(
            "/api",
            openapi_get_routes![
                routes::user_route::register,
                routes::user_route::login,
                routes::flight_route::search_flights,
                routes::flight_route::get_available_seats,
                routes::ticket_route::book_ticket,
                routes::ticket_route::book_seat_for_ticket,
            ],
        )
        .mount("/swagger", make_swagger_ui(&swagger_ui()))
        .attach(AdHoc::on_response("CORS", |_, res| {
            Box::pin(async move {
                res.set_header(rocket::http::Header::new(
                    "Access-Control-Allow-Origin",
                    "*",
                ));
            })
        }))
}
