# Rust Project - Airline Reservation System

## Introduction

This project is a simple airline reservation system built with Rust, Rocket, and SQLX. It allows users to register, login, and manage their reservations.

## Local Development

1. Add a `.env` file in the root directory and add the following:
    ```
    DATABASE_URL="mysql://root:<<PASSWORD>>@localhost:3306/airline_reservation_system"
    JWT_SECRET=your_secret_key_here
    ROCKET_ADDRESS=127.0.0.1
    ROCKET_PORT=8000
    ```

2. Run the following command to start the server:
    ```
    cargo build
    cargo run
    ```

3. Access the API documentation at <http://localhost:8000/swagger/index.html>
