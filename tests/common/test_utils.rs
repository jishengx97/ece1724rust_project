use dotenv::dotenv;
use once_cell::sync::OnceCell;
use sqlx::mysql::MySqlPool as Pool;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Error;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

static TEST_DB: OnceCell<Mutex<Option<TestDb>>> = OnceCell::new();
static DB_NAME: OnceCell<String> = OnceCell::new();

#[derive(Debug)]
pub struct TestDb {
    pub db_name: String,
}

// Create a connection pool without a database, used to create a new database
async fn create_connection_pool_without_db() -> Result<Pool, Error> {
    dotenv().ok();
    let db_url =
        env::var("ADMIN_DATABASE_URL").expect("ADMIN_DATABASE_URL must be set in .env file");

    let base_url = db_url.split("/").collect::<Vec<&str>>()[..3].join("/");

    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&base_url)
        .await
}

// Create a connection pool with a test database
async fn create_connection_pool_with_db(db_name: &str) -> Result<Pool, Error> {
    dotenv().ok();
    let db_url =
        env::var("ADMIN_DATABASE_URL").expect("ADMIN_DATABASE_URL must be set in .env file");

    let base_url = db_url.split("/").collect::<Vec<&str>>()[..3].join("/");

    MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&format!("{}/{}", base_url, db_name))
        .await
}

impl TestDb {
    // Get the database instance - Setup function to initialize the test database for each test
    pub async fn get_instance(file_path: &str) -> Result<Pool, Error> {
        let test_name = file_path
            .split(['/', '\\']) // Handle both Unix and Windows paths
            .last()
            .unwrap_or(file_path)
            .trim_end_matches(".rs");

        // Try to get the database instance
        let test_db = TEST_DB.get_or_init(|| Mutex::new(None));
        let mut guard = test_db.lock().await;

        // If the database instance does not exist, create it
        if guard.is_none() {
            println!("Creating new database instance for {}", test_name);
            *guard = Some(Self::setup_database(test_name).await?);
        }

        // Save the database name
        let db_name = guard.as_ref().unwrap().db_name.clone();
        drop(guard);

        // Create a new connection pool for each test
        println!("Creating new connection pool");
        create_connection_pool_with_db(&db_name).await
    }

    // Setup function to initialize the test database for each test
    async fn setup_database(test_name: &str) -> Result<Self, Error> {
        // Create a unique database name by timestamp for each test
        let db_name = DB_NAME
            .get_or_init(|| {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let name = format!("airline_test_{}_{}", test_name, timestamp);
                println!("Generated database name: {}", name);
                name
            })
            .clone();

        println!("Setting up database: {}", db_name);
        let admin_pool = create_connection_pool_without_db().await?;

        println!("Creating fresh database");
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await?;

        // Create a connection pool with the new database
        let pool = create_connection_pool_with_db(&db_name).await?;
        println!("Initializing tables");
        Self::create_tables(&pool).await?;
        println!("Inserting initial data");
        Self::insert_initial_data(&pool).await?;

        Ok(Self { db_name })
    }

    async fn create_tables(pool: &Pool) -> Result<(), Error> {
        let tables = vec![
            "CREATE TABLE IF NOT EXISTS aircraft (
                aircraft_id INT NOT NULL PRIMARY KEY,
                capacity INT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS user (
                id INT AUTO_INCREMENT PRIMARY KEY,
                username CHAR(255) NOT NULL,
                password CHAR(255) NOT NULL,
                role ENUM('ADMIN', 'USER') DEFAULT 'USER' NOT NULL,
                CONSTRAINT user_username_uindex UNIQUE (username)
            )",
            "CREATE TABLE IF NOT EXISTS customer_info (
                id INT NOT NULL PRIMARY KEY,
                name CHAR(255) NOT NULL,
                birth_date DATE NOT NULL,
                gender ENUM('male', 'female') NOT NULL,
                CONSTRAINT customer_info_user_id_fk
                    FOREIGN KEY (id) REFERENCES user(id)
                    ON DELETE CASCADE
            )",
            "CREATE TABLE IF NOT EXISTS flight_route (
                flight_number INT NOT NULL PRIMARY KEY,
                departure_city CHAR(255) NOT NULL,
                destination_city CHAR(255) NOT NULL,
                departure_time TIME NOT NULL,
                arrival_time TIME NOT NULL,
                aircraft_id INT NOT NULL,
                overbooking DECIMAL(4,2) DEFAULT 0.00 NOT NULL,
                start_date DATE NOT NULL,
                end_date DATE NULL,
                CONSTRAINT flight_route_aircraft_aircraft_id_fk
                    FOREIGN KEY (aircraft_id) REFERENCES aircraft(aircraft_id)
                    ON UPDATE CASCADE ON DELETE CASCADE
            )",
            "CREATE TABLE IF NOT EXISTS flight (
                flight_id INT AUTO_INCREMENT PRIMARY KEY,
                flight_number INT NOT NULL,
                flight_date DATE NOT NULL,
                available_tickets INT NOT NULL,
                version INT NULL,
                CONSTRAINT flight_flight_route_flight_number_fk
                    FOREIGN KEY (flight_number) REFERENCES flight_route(flight_number)
                    ON UPDATE CASCADE ON DELETE CASCADE
            )",
            "CREATE TABLE IF NOT EXISTS seat_info (
                flight_id INT NOT NULL,
                seat_number INT NOT NULL,
                seat_status ENUM('AVAILABLE', 'UNAVAILABLE', 'BOOKED') DEFAULT 'AVAILABLE' NOT NULL,
                version INT DEFAULT 0 NOT NULL,
                PRIMARY KEY (flight_id, seat_number),
                CONSTRAINT seat_info_flight_flight_id_fk
                    FOREIGN KEY (flight_id) REFERENCES flight(flight_id)
                    ON DELETE CASCADE
            )",
            "CREATE TABLE IF NOT EXISTS ticket (
                id INT AUTO_INCREMENT PRIMARY KEY,
                customer_id INT NOT NULL,
                flight_id INT NOT NULL,
                seat_number INT NULL,
                flight_date DATE NOT NULL,
                flight_number INT NOT NULL,
                CONSTRAINT ticket_customer_info_id_fk
                    FOREIGN KEY (customer_id) REFERENCES customer_info(id)
                    ON DELETE CASCADE,
                CONSTRAINT ticket_flight_id_fk
                    FOREIGN KEY (flight_id) REFERENCES flight(flight_id)
                    ON DELETE CASCADE,
                CONSTRAINT ticket_seat_info_flight_id_seat_number_fk
                    FOREIGN KEY (flight_id, seat_number) 
                    REFERENCES seat_info(flight_id, seat_number)
            )",
        ];

        for create_sql in tables {
            sqlx::query(create_sql).execute(pool).await?;
        }

        Ok(())
    }

    async fn insert_initial_data(_pool: &Pool) -> Result<(), Error> {
        // No global test data needed
        Ok(())
    }

    //TODO: Maybe add more functions to help setup to create default test data

    // Teardown function to drop database after test run (not after each test)
    pub fn cleanup_database_sync() -> Result<(), Box<dyn std::error::Error>> {
        dotenv().ok();

        // Use .env file to get the admin database url
        let db_url = env::var("ADMIN_DATABASE_URL").expect("DATABASE_URL must be set in .env file");
        let url_parts: Vec<&str> = db_url.split("://").nth(1).unwrap().split("@").collect();
        let auth = url_parts[0].split(":").collect::<Vec<&str>>();
        let username = auth[0];
        let password = auth[1];

        // Get the database name and drop the database
        if let Some(db_name) = DB_NAME.get() {
            let output = std::process::Command::new("mysql")
                .arg("-u")
                .arg(username)
                .arg(format!("-p{}", password))
                .arg("-e")
                .arg(format!("DROP DATABASE IF EXISTS {};", db_name))
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to drop test database: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }
        Ok(())
    }
}
