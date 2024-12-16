# ECE1724 Final Report: Backend for Airline Booking Systems

Guanhong Wu 1002377475 <guanhong.wu@mail.utoronto.ca>

Shengxiang Ji 1002451232 <shengxiang.ji@mail.utoronto.ca>

## Table of Contents

- [Motivation](#motivation)
- [Objective](#objective)
- [Features](#features)
  - [User Service API](#user-service-api)
  - [Flight Service API](#flight-service-api)
  - [Ticket Service API](#ticket-service-api)
  - [Utils](#utils)
- [Reproducibility Guide](#reproducibility-guide)
- [User's Guide](#users-guide)
- [Developer's Guide](#developers-guide)
- [Contributions](#contributions)
- [Lessons Learned](#lessons-learned)

## Motivation

According to the United States Department of Transportation, over 900 million air travelers were transported by US airlines in 2023 alone [1]. Therefore a high-performance and secure airline booking system is essential. We are driven to implement a web server backend for an airline booking system that provides REST API services in Rust because we believe that the particular advantages of Rust as a language lend themselves well to this application space. Web servers these days are usually built on Spring Boot for Java or .NET/ASP .NET Core frameworks. Those are all great tools, but Rust provides some advantages that help to solve problems inherent to airline booking systems.

Rust, a fast, compiled language whose performance comes as close to C or C++ as possible, is very suitable for an airline booking system with such a high request count. This is because the same ticket may be booked by multiple customers or the same seat and the system has to respond within milliseconds otherwise data will be inconsistent.

Rust also provides memory safety, without needing a garbage collector, therefore making it less prone to errors and security issues. This safety is best in systems that want accuracy and reliability of data. Because of the design of Rust, it is easier to write concurrent code and deal  with the issues of data races, deadlocks, and so on.

Rust offers mature web frameworks: Rocket, Actix Web which support building web applications. Rust has crates like Diesel and SQLx to interact with databases and it can work with MySQL. They provide an extensive foundation to create a scalable and performant solution for an airline reservation system.

Nonetheless, it seems that SQLx do not provide built-in support for optimistic locking, a mechanism that is useful for maintaining data consistency in high-concurrency systems. For example, when multiple customers try to book the last available seat at the same time, the system needs to handle the conflicts to ensure the seat is only booked by one customer and another customer receives an error message to avoid double booking on the same seat.

Next, we want to take the issues described above and write implement optimistic locking with SQLx. This will assist the system to be capable of being aware of inconsistency while multiple transactions are running at the same time and rollback transactions, if needed. Doing this will ensure data consistency but it will not degrade the performance of the airline booking system.

We aim to provide a demo for optimistic locking in database interactions using this project, filling a gap in the Rust ecosystem. We hope that this can showcase Rust as a language for constructing high-performance, safe and robust web servers for complicated systems, such as airline booking platforms.

## Objective

We are building a Rust-based backend for an airline ticket booking system. The aim is to create a powerful and efficient REST API that allows users to do various booking activities like searching flights, booking seats, and making reservations. By using Rust's features, along with the Rocket framework for web development and SQLx for database interactions, we plan to fill gaps in Rust's ecosystem, especially around data consistency for concurrent transactions through optimistic locking.

Rust has strong frameworks, like Rocket, for handling REST API requests. We'll use MySQL as our database, connecting through Rust library SQLx. These tools help us build a system that meets the needs of an airline booking platform with high performance and safety standards.

## Features

Our API system provides comprehensive endpoints for user management and flight operations. All responses are in JSON format and require appropriate error handling.

### User Service API

The User Service handles user authentication and registration operations, providing secure access to the system.

#### Register User (`POST /api/register`)

Creates a new user account in the system.

**Request Body:**

```json
{
  "username": "john_doe",
  "password": "secure_password123",
  "name": "John Doe",
  "birth_date": "1990-01-01",
  "gender": "M",
  "role": "USER"  // Optional, defaults to "USER"
}
```

**Response (200 OK):**

```json
{
  "user_id": 12345,
  "status": "success"
}
```

**Error Handling:**

- `400 Bad Request`: Invalid input data
  - Gender is not male or female
  - Password cannot be hashed
- `409 Conflict`: Username already exists
- `422 Unprocessable Entity`: Missing required fields or incorrect format

#### User Login (`POST /api/login`)

Authenticates a user and provides a JWT token for subsequent requests. The token is valid for 24 hours and records the user_id of the user that is logging in for future API requests.

**Request Body:**

```json
{
  "username": "john_doe",
  "password": "secure_password123"
}
```

**Response (200 OK):**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user_id": 12345
}
```

**Error Handling:**

- `401 Unauthorized`: Invalid credentials (username or password is incorrect)
- `422 Unprocessable Entity`: Missing required fields or incorrect format

### Flight Service API

The Flight Service provides functionality to search flights and check seat availability.

#### Search Flights (`GET /api/flights/search`)

Searches for available flights based on specified criteria.

**Query Parameters:**

- Required:
  - `departure_city`: String (e.g., "YYZ")
  - `destination_city`: String (e.g., "JFK")
  - `departure_date`: YYYY-MM-DD (e.g., "2024-10-25")
- Optional:
  - `end_date`: YYYY-MM-DD (e.g., "2024-11-20")

**Example Request:**

```
GET /api/flights/search?departure_city=YYZ&destination_city=JFK&departure_date=2024-10-25
```

**Response (200 OK):**

```json
{
  "flights": [
    {
      "flight_id": 123,
      "flight_number": 123,
      "departure_city": "YYZ",
      "destination_city": "JFK",
      "departure_time": "10:00:00",
      "arrival_time": "11:15:00",
      "available_tickets": 50,
      "flight_date": "2024-10-20"
    },
    ...
  ]
}
```

**Error Handling:**

- `400 Bad Request`: Invalid date format
  - Date format is not YYYY-MM-DD
- `401 Unauthorized`: Invalid or missing JWT token
- `422 Unprocessable Entity`: Missing required fields or incorrect format

#### Get Available Seats (`GET /api/flights/availableSeats`)

Retrieves available seats for a specific flight.

**Query Parameters:**

- Required:
  - `flight_number`: Integer (e.g., 123)
  - `flight_date`: YYYY-MM-DD (e.g., "2024-06-15")

**Example Request:**

```
GET /api/flights/availableSeats?flight_number=123&flight_date=2024-06-15
```

**Response (200 OK):**

```json
{
  "available_seats": [1, 2, 3, 5, 8, 13, 21]
}
```

**Error Handling:**

- `400 Bad Request`: Invalid date format
- `401 Unauthorized`: Invalid or missing JWT token
- `404 Not Found`: Flight not found
- `422 Unprocessable Entity`: Missing required fields or incorrect format

### Ticket Service API

The Ticket Service handles flight ticket booking operations and booking history queries. It supports ticket booking with seat selection and viewing booking history.

#### Book Ticket (`POST /api/tickets/book`)

Books tickets for one or multiple flights in one request. User also can choose to book a preferred seat for each flight. If the seat is already booked or unavailable, the ticket still can be booked without the preferred seat.
If user tries to book a ticket with multiple flights and some of the flights are not available or the user already has a ticket for some of the flights, all tickets will not be booked.
This API is implemented with optimistic locking to ensure data consistency when multiple users try to book the same ticket and/or seat at the same time. The optimistic locking will retry the booking processs until the booking is successful or the booking is failed due to out of stock.

**Request Body Example:**

```json
{
  "flights": [
    {
      "flight_number": 123,
      "flight_date": "2024-06-15",
      "preferred_seat": 12  // Optional
    },
    {
      "flight_number": 456,
      "flight_date": "2024-06-16",
      "preferred_seat": null // Optional
    },
    ...
  ]
}
```

**Response (200 OK):**

```json
{
  "booking_status": "Confirmed",
  "flight_bookings": [
    {
      "ticket_id": 789,
      "flight_details": "Flight 123 on 2024-06-15",
      "seat_number": 12
    },
    {
      "ticket_id": 790,
      "flight_details": "Flight 456 on 2024-06-16",
      "seat_number": null
    }
  ]
}
```

**Error Handling:**

- `400 Bad Request`:
  - Flight(s) does not exist
  - Flight(s) already booked by current user
  - Flight(s) is fully booked
- `401 Unauthorized`: Invalid or missing JWT token
- `422 Unprocessable Entity`: Missing required fields or incorrect format

#### Book/Change Seat (`POST /api/tickets/seat/book`)

Books or changes a seat for an existing ticket.
This API will handle the request to help user book a seat for a flight and release the old seat if the user already holds a seat for the flight.
This API is implemented with optimistic locking to ensure data consistency when multiple users try to book the same seat at the same time.

**Request Body:**

```json
{
  "flight_number": 123,
  "flight_date": "2024-06-15",
  "seat_number": 15
}
```

**Response (200 OK):**

```json
{
  "success": true
}
```

**Error Handling:**

- `400 Bad Request`:
  - Seat already booked
  - No ticket of this flight for current user
  - Seat is not available
- `401 Unauthorized`: Invalid or missing JWT token
- `404 Not Found`:
  - Flight not found
  - Seat not found
- `422 Unprocessable Entity`: Missing required fields or incorrect format
  
#### Get Booking History (`GET /api/history`)

Retrieves the booking history that includes all tickets for the authenticated user.

**Response (200 OK):**

```json
{
  "flights": [
    {
      "flight_number": 123,
      "seat_number": "15",
      "departure_city": "YYZ",
      "destination_city": "JFK",
      "flight_date": "2024-10-20",
      "departure_time": "10:00:00",
      "arrival_time": "11:15:00"
    },
    ...
  ]
}
```

**Error Handling:**

- `401 Unauthorized`: Invalid or missing JWT token

### Utils

#### Swagger Integration

We integrated Swagger UI to help developers explore and test the APIs. The Swagger UI provides an interactive interface to view all API endpoints and schemas at <http://localhost:8000/swagger/index.html>. Due to current limitations in Rust's OpenAPI implementation, JWT token authentication cannot be included in requests' header through Swagger. Therefore, only the register and login endpoints can be fully tested via Swagger UI. However, it remains a valuable tool for API documentation and exploration.

![Swagger UI API Screenshot](media/swagger_api.png)
![Swagger UI Schemas Screenshot](media/swagger_schemas.png)

#### Database Initialization Script

The `create_database.sql` script helps initialize the MySQL database with the required schema. It creates the following key tables:

- `aircraft`: Stores aircraft information with default entries for models 737, 777, 320, 900, and 200
- `user` and `customer_info`: Manages user authentication and customer details
- `flight_route`: Contains flight route information including cities and schedules
- `flight`: Tracks individual flights and available tickets
- `seat_info`: Manages seat availability status
- `ticket`: Records ticket bookings and seat assignments

#### Flight Data Generation Script

Since we haven't implemented administrative APIs for adding flights, we created a Python script (`create_flight_script.py`) to populate the database with sample flight data. This script adds default flight routes and generates corresponding flights for testing and demonstration purposes. It creates two flight routes between major cities like JFK-YYZ and LAX-JFK with realistic schedules and seat configurations.

## Reproducibility Guide

### 1. Install and configure the `MySQL` database

#### Ubuntu Installation

```bash
sudo apt update
sudo apt install mysql-server
sudo systemctl start mysql.service
```

#### macOS Installation

```zsh
brew install mysql
brew services start mysql
```

#### Configure the database after installation

```bash
# Login as root user
sudo mysql -u root
# Enter system root password as required
# Update the root user password (replace <some secret password> with the actual password)
mysql> ALTER USER 'root'@'localhost' IDENTIFIED WITH caching_sha2_password  by '<some secret password>;
mysql> FLUSH PRIVILEGES;
mysql> quit
# test logging in as the root user with the above password
mysql -u root -p
```

### 2. Create a text file named `.env` in the project directory, and put the following lines in it (replace \<your secret password\> with the actual password set in step 1)

```bash
DATABASE_URL="mysql://root:<your secret password>@localhost:3306/airline_reservation_system"
ADMIN_DATABASE_URL="mysql://root:<your secret password>@localhost:3306/mysql"
JWT_SECRET=your_secret_key_here
ROCKET_ADDRESS=127.0.0.1
ROCKET_PORT=8000
```

### 3. Setup the database

```bash
# Replace <your secret password> with the actual password
mysql -u root -p"<your secret password>" < util/create_database.sql
```

### 4. Insert some testing data into the database

```bash
# Install necessary python packages
pip install mysql-connector-python python-dotenv
python util/create_flight_script.py
```

### 5. Compile and run the rust project

```bash
cargo build
cargo run
```

## User's Guide

### 1. To register an user, send a POST request to route `api/register`

```bash
curl "http://localhost:8000/api/register/" \
  --json '{"username": "<your username>", "password": "<your password>", "name": "<your name>", "birth_date": "<YYYY-MM-DD>", "gender": "[male|female]"}'
```

On success, it will return the registration status and the user_id:

```console
user@system:~$ curl "http://localhost:8000/api/register/" \
  --json '{"username": "user1", "password": "000000", "name": "Jane Doe", "birth_date": "2000-01-01", "gender": "male"}'
{"user_id":1,"status":"success"}
```

### 2. To login, send a POST request to route `api/login`

```bash
curl "http://localhost:8000/api/login/" \
  --json '{"username": "<your username>", "password": "<your password>"}'
```

On success, it will return the user_id of the user that is loggin in, as well as a JWT token for authorizing future requests, valid for 24 hours. Take a note of the token.

```console
user@system:~$ curl "http://localhost:8000/api/login/" \
  --json '{"username": "user1", "password": "000000"}'
{"token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s","user_id":1}
```

### 3. To search for flights, send a GET request to route `api/flights/search`. Supply the departure city, destination city, and the date of travel. Optionally, you can supply the search end date to search for all the matching flights between the departure date and end date

```bash
curl \
  --header "Authorization: Bearer <your JWT token>" \
  "http://localhost:8000/api/flights/search?departure_city=<departure city>&destination_city=<destination city>&departure_date=<YYYY-MM-DD>[&end_date=<YYYY-MM-DD>]"
```

On success, the response will contain a list of flights that matches the search criteria.

```console
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  "http://localhost:8000/api/flights/search?departure_city=JFK&destination_city=YYZ&departure_date=2024-10-24"
{"flights":[{"flight_id":1,"flight_number":590,"departure_city":"JFK","destination_city":"YYZ","departure_time":"07:20:00","arrival_time":"08:50:00","available_tickets":149,"flight_date":"2024-10-24"}]}
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  "http://localhost:8000/api/flights/search?departure_city=JFK&destination_city=YYZ&departure_date=2024-10-24&end_date=2024-10-25"
{"flights":[{"flight_id":1,"flight_number":590,"departure_city":"JFK","destination_city":"YYZ","departure_time":"07:20:00","arrival_time":"08:50:00","available_tickets":149,"flight_date":"2024-10-24"},{"flight_id":2,"flight_number":590,"departure_city":"JFK","destination_city":"YYZ","departure_time":"07:20:00","arrival_time":"08:50:00","available_tickets":149,"flight_date":"2024-10-25"}]}
```

### 4. To search for available seats for a flight, send a GET request to route `api/flights/availableSeats`. Supply the flight number and flight date of the flight

```bash
curl \
  --header "Authorization: Bearer <your JWT token>" \
  "http://localhost:8000/api/flights/availableSeats?flight_number=<flight number>&flight_date=<YYYY-MM-DD>"
```

On success, the response will list all the remaining available seats on the flight.

```console
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  "http://localhost:8000/api/flights/availableSeats?flight_number=590&flight_date=2024-10-24"
{"available_seats":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95,96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,146]}
```

### 5. To book tickets for flights, send a POST requests to route `api/tickets/book`. One or more flights can be booked with a single request, and the prefered seats can be optionally specified

```bash
curl \
  --header "Authorization: Bearer <your JWT token>" \
  --json '{"flights": [{ "flight_number": <flight number 1>, "flight_date": "<YYYY-MM-DD>"[, "preferred_seat": <preferred seat 1>]}, { "flight_number": <flight number 2>, "flight_date": "<YYYY-MM-DD>"[, "preferred_seat": <preferred seat 2>]}]}' \
  "http://localhost:8000/api/tickets/book"
```

On success, the response will contain whether the booking is successfully confirmed. It will also contain the details of the booked flight, as well as the confirmed seat numbers on those flights.

```console
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  --json '{"flights": [{ "flight_number": 590, "flight_date": "2024-10-24"}]}' \
  "http://localhost:8000/api/tickets/book"
{"booking_status":"Confirmed","flight_bookings":[{"flight_details":"Flight 590 on 2024-10-24","seat_number":null,"ticket_id":1}]}
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  --json '{"flights": [{ "flight_number": 590, "flight_date": "2024-10-25", "preferred_seat": 1}]}' \
  "http://localhost:8000/api/tickets/book"
{"booking_status":"Confirmed","flight_bookings":[{"flight_details":"Flight 590 on 2024-10-25","seat_number":1,"ticket_id":2}]}
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  --json '{"flights": [{ "flight_number": 590, "flight_date": "2024-10-26", "preferred_seat": 17}, { "flight_number": 1284, "flight_date": "2024-10-26", "preferred_seat": 24}]}' \
  "http://localhost:8000/api/tickets/book"
{"booking_status":"Confirmed","flight_bookings":[{"flight_details":"Flight 590 on 2024-10-26","seat_number":17,"ticket_id":3},{"flight_details":"Flight 1284 on 2024-10-26","seat_number":24,"ticket_id":4}]}
```

### 6. To select or update the seat for confirmed flights, send a POST request to route `api/tickets/seat/book`

```bash
curl \
  --header "Authorization: Bearer <your JWT token>" \
  --json '{"flight_number": <flight number>, "flight_date": "<YYYY-MM-DD>", "seat_number": <seat number>}' \
  "http://localhost:8000/api/tickets/seat/book"
```

On success, the reponse will indicate the seat selection succeeded.

```console
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  --json '{"flight_number": 590, "flight_date": "2024-10-24", "seat_number": 24}' \
  "http://localhost:8000/api/tickets/seat/book"
{"success":true}
```

### 7. To query the flight booking histories, send a GET request to route `api/history`

```bash
curl \
  --header "Authorization: Bearer <your JWT token>" \
  "http://localhost:8000/api/history"
```

On success, the reponse will contain a list of confirmed flight tickets the user has booked.

```console
user@system:~$ curl \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NzQ0M30.arPG4Jp2-KDs0V6-El2LhUR6bsW2xD0gxe28htl8I-s" \
  "http://localhost:8000/api/history"
{"flights":[{"flight_number":590,"seat_number":"17","departure_city":"JFK","destination_city":"YYZ","flight_date":"2024-10-26","departure_time":"07:20:00","arrival_time":"08:50:00"},{"flight_number":1284,"seat_number":"24","departure_city":"LAX","destination_city":"JFK","flight_date":"2024-10-26","departure_time":"23:55:00","arrival_time":"07:00:00"},{"flight_number":590,"seat_number":"1","departure_city":"JFK","destination_city":"YYZ","flight_date":"2024-10-25","departure_time":"07:20:00","arrival_time":"08:50:00"},{"flight_number":590,"seat_number":"24","departure_city":"JFK","destination_city":"YYZ","flight_date":"2024-10-24","departure_time":"07:20:00","arrival_time":"08:50:00"}]}
```

## Developer's Guide

### Testing

The project uses the cargo unit test infrastructure to ensure the functionality and performance of the package. To run the tests, simply run:

```bash
cargo test
```

You will see console output similar to:

![Cargo Test output](media/cargo_test_output.png)

Or to enable debug output and progress reporting, run:

```bash
cargo test -- --nocapture
```

In the `tests/` folder, there are tests for different services of the package. Each test class owns their own copies of the database, and test data is inserted separately for each test. This ensures the results of the tests are independent of each other, and eliminates risks of data pollution between tests.

- The `user_service_test.rs` contains tests that ensures the user authencation functionalities are working correctly. It includes tests that ensure user registration requests can be correctly processed, tests that ensure requests with duplicated usernames can be correctly rejected, tests that ensure users can login with the correct password and vice versa, and tests that ensure non-existent users cannot login.
- The `flight_service_test.rs` contains tests that ensures flight data can successfully be queried. It includes tests that ensures flights can be correctly searched given either a single date or a date range, tests that ensures available seat status can correctly be queried whether or not flight is partially booked, and tests that ensure non-existent flight query returns an error.
- The `ticket_service_test.rs` contains tests for ticket booking and seat selections, and ensures flights and seats are not double-booked even in concurrent request environments. Specifically, for both flight booking and selection services, the test will send 10 concurrent requests when only one flight ticket or seat is available, or send 20 concurrent requests when only five flight tickets or setas are available. At the end, the test will check the sold tickets and seats exactly matches with the remaining tickets or seats.
- The `throughput_test.rs` generates a large number of random requests to the system, to ensure the system is able to maintain a high throughput even when the requests are highly concurrent. It will have 100 users generate 2000 random concurrent requests, and display the system throughput (requests/second) at the end. On a personal desktop with an i9-9900k CPU, the system can achieve over 140 requests/second.

![test_massive_concurrent_booking](media/throughput_test.PNG)

## Contributions

### Guanhong Wu

- Set up project framework with Rocket, SQLx, and Swagger (rocket_okapi)
- Designed and implemented MySQL database schema
- Implemented user registration and login functionality
- Developed seat booking system with optimistic locking to handle concurrent seat selections
- Created flight search and seat availability APIs
- Established testing framework with isolated database setup/teardown for each test class
- Wrote test cases for user authentication, flight search, seat availability, and ticket retrieval

### Shengxiang Ji

- Created Python scripts for testing and demonstration data generation
- Implemented ticket booking system with support for multiple flights in one request and optimistic locking
- Implemented booking history APIs
- Developed concurrent booking tests to verify system behavior under multiple user scenarios
- Created throughput tests to evaluate web server performance
- Wrote comprehensive Reproducibility Guide for environment setup and server deployment
- Prepared demonstration video

## Lessons Learned

### Project Management

- Early API documentation could have prevented multiple redesigns of request/response structures. For example, our ticket booking API was modified three times to accommodate new requirements, such as adding preferred seat selection and multiple flight bookings in one request.
- Pre-defined API specifications would have helped us identify these needs earlier. This would be particularly beneficial if frontend development were to be implemented, as it would ensure smooth integration between frontend and backend components.

### Testing Strategy

- Building tests alongside features helped catch critical issues early, particularly in concurrent booking scenarios. For instance, our concurrent booking tests revealed a race condition in seat selection that could have led to double bookings in production. This was identified and fixed during development rather than after deployment.
- Additionally, having comprehensive tests from the beginning would have eliminated the need for manual testing during minor code modifications, significantly improving development efficiency.

## Video Demo

The video demo of this project can be found [here](video_demo.mp4).

> ### References
> [1] BUREAU OF TRANSPORTATION STATISTICS, "Passengers Traffic," Bts.gov, <https://www.transtats.bts.gov/Data_Elements.aspx?Data=1> (accessed Nov. 3, 2024).
