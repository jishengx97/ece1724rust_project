# ECE1724 Final Report: Backend for Airline Booking Systems

Guanhong Wu 1002377475  guanhong.wu@mail.utoronto.ca

Shengxiang Ji 1002451232 shengxiang.ji@mail.utoronto.ca

## Motivation

According to the United States Department of Transportation, over 900 million air travelers were transported by US airlines in 2023 alone [1]. Therefore a high-performance and secure airline booking system is essential. We are driven to implement a web server backend for an airline booking system that provides REST API services in Rust because we believe that the particular advantages of Rust as a language lend themselves well to this application space. Web servers these days are usually built on Spring Boot for Java or .NET/ASP .NET Core frameworks. Those are all great tools, but Rust provides some advantages that help to solve problems inherent to airline booking systems.

Rust, a fast, compiled language whose performance comes as close to C or C++ as possible, is very suitable for an airline booking system with such a high request count. This is because the same ticket may be booked by multiple customers or the same seat and the system has to respond within milliseconds otherwise data will be inconsistent.

Rust also provides memory safety, without needing a garbage collector, therefore making it less prone to errors and security issues. This safety is best in systems that want accuracy and reliability of data. Because of the design of Rust, it is easier to write concurrent code and deal  with the issues of data races, deadlocks, and so on.

Rust offers mature web frameworks: Rocket, Actix Web which support building web applications. Rust has crates like Diesel and SQLx to interact with databases and it can work with MySQL. They provide an extensive foundation to create a scalable and performant solution for an airline reservation system.

Nonetheless, it seems that SQLx both do not provide built-in support for optimistic locking, a mechanism that is useful for maintaining data consistency in high-concurrency systems. For example, when multiple customers try to book the last available seat at the same time, the system needs to handle the conflicts to ensure the seat is only booked by one customer and another customer receives an error message to avoid double booking on the same seat.

Next, we want to take the issues described above and write implement optimistic locking with SQLx. This will assist the system to be capable of being aware of inconsistency while multiple transactions are running at the same time and rollback transactions, if needed. Doing this will ensure data consistency but it will not degrade the performance of the airline booking system.

We aim to provide a demo for optimistic locking in database interactions using this project, filling a gap in the Rust ecosystem. We hope that this can showcase Rust as a language for constructing high-performance, safe and robust web servers for complicated systems, such as airline booking platforms.

## Objective

We are building a Rust-based backend for an airline ticket booking system. The aim is to create a powerful and efficient REST API that allows users to do various booking activities like searching flights, booking seats, and making reservations. By using Rust's features, along with the Rocket framework for web development and SQLx for database interactions, we plan to fill gaps in Rust’s ecosystem, especially around data consistency for concurrent transactions through optimistic locking.

Rust has strong frameworks, like Rocket, for handling REST API requests. We’ll use MySQL as our database, connecting through Rust library SQLx. These tools help us build a system that meets the needs of an airline booking platform with high performance and safety standards.

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
- `422 Unprocessable Entity`: Missing required fields

#### User Login (`POST /api/login`)
Authenticates a user and provides a JWT token for subsequent requests.

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
- `422 Unprocessable Entity`: Missing required fields

### Flight Service API

The Flight Service provides functionality to search flights and check seat availability.

#### Search Flights (`GET /api/flights/search`)
Searches for available flights based on specified criteria.

**Query Parameters:**
- Required:
  - `departure_city`: String (e.g., "New York")
  - `destination_city`: String (e.g., "London")
  - `departure_date`: YYYY-MM-DD (e.g., "2024-06-15")
- Optional:
  - `end_date`: YYYY-MM-DD (e.g., "2024-06-20")

**Example Request:**
```
GET /api/flights/search?departure_city=New York&destination_city=London&departure_date=2024-06-15
```

**Response (200 OK):**
```json
{
  "flights": [
    {
      "flight_id": 123,
      "flight_number": "AA123",
      "departure_city": "New York",
      "destination_city": "London",
      "departure_time": "10:00:00",
      "arrival_time": "22:00:00",
      "available_tickets": 50,
      "flight_date": "2024-06-15"
    }
  ]
}
```

**Error Handling:**
- `400 Bad Request`: Invalid date format 
  -  Date format is not YYYY-MM-DD
- `401 Unauthorized`: Invalid or missing JWT token
- `422 Unprocessable Entity`: Missing required fields

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
- `422 Unprocessable Entity`: Missing required fields

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

### 2. Setup the database

```bash
# Replace <some secret password> with the actual password
mysql -u root -p"<your secret password>" < util/create_database.sql
```

### 3. Insert some testing data into the database

```bash
# Install necessary python packages
pip install mysql-connector-python python-dotenv
python util/create_flight_script.py
```

### 4. Compile and run the rust project

```bash
cargo build
cargo run
```

## User's Guide

### 1. To register an user, send a POST request to route api/register/, replacing the fields in angle brackets with real values:

```bash
curl "http://localhost:8000/api/register/" \
  --json '{"username": "<your username>", "password": "<your password>", "name": "<your name>", "birth_date": "<your birthdate>", "gender": "[male|female]"}'
```

On success, it will return the registration status and the user_id:
```console
user@system:~$ curl "http://localhost:8000/api/register/" \
  --json '{"username": "user1", "password": "000000", "name": "Jane Doe", "birth_date": "2000-01-01", "gender": "male"}'
{"user_id":1,"status":"success"}
```

### 2. To login, send a POST request to route api/login/, replacing the fields in angle brackets with real values:

```bash
curl "http://localhost:8000/api/login/" \
  --json '{"username": "user1", "password": "000000"}'
```

On success, it will return the user_id of the user that is loggin in, as well as a JWT token for authorizing future requests, valid for 24 hours:
```console
user@system:~$ curl "http://localhost:8000/api/login/" \
  --json '{"username": "user1", "password": "000000"}'
{"token":"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOjEsImV4cCI6MTczNDE0NTQ1OH0.BysiTTXpzrt5vBw4WtZvVuq1EfwagwRQhGRKc94fFkY","user_id":1}
```

```bash
curl -H 'Accept: application/json' -H "Authorization: Bearer ${TOKEN}" https://{hostname}/api/myresource
```
