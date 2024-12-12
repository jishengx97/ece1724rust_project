# ECE1724 Final Report: Backend for Airline Booking Systems

Guanhong Wu 1002377475  guanhong.wu@mail.utoronto.ca   
Shengxiang Ji 1002451232

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

### User Authentication API
- **User Registration** (`POST api/register`)
  - Enables new users to create accounts
  - Supports registration with basic information including username, password, name, birth date, and gender, and role (optional,the default role is user)
  - Implements password encryption using bcrypt to store encrypted password in database
  - Prevents duplicate username registration
  
- **User Login** (`POST api/login`) 
  - Provides user login functionality by username and password, if the username or password is incorrect, it will return an error message with status code 401
  - Validates user credentials and compare the password with the encrypted password in database
  - Returns JWT token upon successful login for subsequent request authentication and the JWT token will contain the user id and expiration time with 24 hours

## Reproducibility

### 1. Install the `mysql` database

#### Ubuntu Installation
```bash
sudo apt update
sudo apt install mysql-server
sudo systemctl start mysql.service
# login as root user
sudo mysql -u root
# enter system root password as required
# update the root user password (replace <some secret password> with the actual password)
mysql> ALTER USER 'root'@'localhost' IDENTIFIED WITH caching_sha2_password  by '<some secret password>;
mysql> FLUSH PRIVILEGES;
mysql> quit
# test logging in as the root user with the above password
mysql -u root -p
```

#### macOS Installation
```zsh
brew install mysql
brew services start mysql
# login as root user
sudo mysql -u root
# enter system root password as required
# update the root user password (replace <some secret password> with the actual password)
mysql> ALTER USER 'root'@'localhost' IDENTIFIED WITH caching_sha2_password  by '<some secret password>;
mysql> FLUSH PRIVILEGES;
mysql> quit
# test logging in as the root user with the above password
mysql -u root -p
```