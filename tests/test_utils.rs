use dotenv::dotenv;
use once_cell::sync::OnceCell;
use sqlx::mysql::MySqlPool as Pool;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::Error;
use std::env;
use tokio::sync::Mutex;

static TEST_DB: OnceCell<Mutex<Option<TestDb>>> = OnceCell::new();

pub struct TestDb {
    pub pool: Pool,
    pub db_name: String,
}

async fn create_connection_pool_without_db() -> Result<Pool, Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    
    let base_url = db_url.split("/").collect::<Vec<&str>>()[..3].join("/");
    
    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&base_url)
        .await
}

async fn create_connection_pool_with_db(db_name: &str) -> Result<Pool, Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    
    let base_url = db_url.split("/").collect::<Vec<&str>>()[..3].join("/");
    
    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&format!("{}/{}", base_url, db_name))
        .await
}

impl TestDb {
    pub async fn get_instance() -> Result<Pool, Error> {
        let test_db = TEST_DB.get_or_init(|| {
            Mutex::new(None)
        });
        
        let mut test_db_guard = test_db.lock().await;
        
        if test_db_guard.is_none() {
            let db = Self::setup_database().await?;
            *test_db_guard = Some(db);
        }
        
        Ok(test_db_guard.as_ref().unwrap().pool.clone())
    }

    async fn setup_database() -> Result<Self, Error> {
        let db_name = format!("airline_test_{}", chrono::Utc::now().timestamp());
        let admin_pool = create_connection_pool_without_db().await?;
        
        sqlx::query(&format!("CREATE DATABASE {}", db_name))
            .execute(&admin_pool)
            .await?;
        
        let pool = create_connection_pool_with_db(&db_name).await?;
        
        Self::create_tables(&pool).await?;
        Self::insert_initial_data(&pool).await?;
        
        Ok(Self { pool, db_name })
    }

    async fn create_tables(pool: &Pool) -> Result<(), Error> {
        let tables = vec![
            // 创建aircraft表
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
            )"
        ];

        for create_sql in tables {
            sqlx::query(create_sql)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    async fn insert_initial_data(pool: &Pool) -> Result<(), Error> {
        let aircrafts = vec![
            "INSERT INTO aircraft (aircraft_id, capacity) VALUES (737, 169)",
            "INSERT INTO aircraft (aircraft_id, capacity) VALUES (777, 400)",
            "INSERT INTO aircraft (aircraft_id, capacity) VALUES (320, 146)",
            "INSERT INTO aircraft (aircraft_id, capacity) VALUES (900, 76)",
            "INSERT INTO aircraft (aircraft_id, capacity) VALUES (200, 50)"
        ];

        for aircraft_sql in aircrafts {
            sqlx::query(aircraft_sql)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    pub async fn cleanup_database() -> Result<(), Error> {
        if let Some(test_db) = TEST_DB.get() {
            if let Some(db) = test_db.lock().await.take() {
                let admin_pool = create_connection_pool_without_db().await?;
                sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db.db_name))
                    .execute(&admin_pool)
                    .await?;
            }
        }
        Ok(())
    }
}