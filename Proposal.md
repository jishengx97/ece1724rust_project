# ECE1724 Project Proposal: Backend for Airline Booking System

Guanhong Wu 1002377475  
Shengxiang Ji 1002451232

## Motivation

According to the United States Department of Transportation, over 900 million air travelers were transported by US airlines in 2023 alone [1]. Therefore a high-performance and secure airline booking system is essential. We are driven to implement a web server backend for an airline booking system that provides REST API services in Rust because we believe that the particular advantages of Rust as a language lend themselves well to this application space. Web servers these days are usually built on Spring Boot for Java or .NET/ASP .NET Core frameworks. Those are all great tools, but Rust provides some advantages that help to solve problems inherent to airline booking systems.

Rust, a fast, compiled language whose performance comes as close to C or C++ as possible, is very suitable for an airline booking system with such a high request count. This is because the same ticket may be booked by multiple customers or the same seat and the system has to respond within milliseconds otherwise data will be inconsistent.

Rust also provides memory safety, without needing a garbage collector, therefore making it less prone to errors and security issues. This safety is best in systems that want accuracy and reliability of data. Because of the design of Rust, it is easier to write concurrent code and deal  with the issues of data races, deadlocks, and so on.

Rust offers mature web frameworks: Rocket, Actix Web which support building web applications. Rust has crates like Diesel and SQLx to interact with databases and it can work with MySQL. They provide an extensive foundation to create a scalable and performant solution for an airline reservation system.

Nonetheless, it seems that Diesel and SQLx both do not provide built-in support for optimistic locking, a mechanism that is useful for maintaining data consistency in high-concurrency systems. For example, when multiple customers try to book the last available seat at the same time, the system needs to handle the conflicts to ensure the seat is only booked by one customer and another customer receives an error message to avoid double booking on the same seat.

Next, we want to take the issues described above and write some custom methods to implement optimistic locking with Diesel or SQLx. This will assist the system to be capable of being aware of inconsistency while multiple transactions are running at the same time and rollback transactions, if needed. Doing this will ensure data consistency but it will not degrade the performance of the airline booking system.

We aim to provide a solution for optimistic locking in database interactions using this project, filling a gap in the Rust ecosystem. We hope that this can showcase Rust as a language for constructing high-performance, safe and robust web servers for complicated systems, such as airline booking platforms.

## Objective and key features

We are building a Rust-based backend for an airline ticket booking system. The aim is to create a powerful and efficient REST API that allows users to do various booking activities like searching flights, booking seats, and making reservations. By using Rust's features, along with the Rocket framework for web development and SQLx (or Diesel) for database interactions, we plan to fill gaps in Rust’s ecosystem, especially around data consistency for concurrent transactions through optimistic locking.

Rust has strong frameworks, like Rocket, for handling REST API requests. We’ll use MySQL as our database, connecting through Rust libraries like Diesel or SQLx. These tools help us build a system that meets the needs of an airline booking platform with high performance and safety standards.

---

### Key Features

1. **Backend Development with Rust and Rocket**
   - **Mandatory Features**
     - **User Registration API (POST)**
       - Allows new users to create accounts with a username and password.
       - Stores user information securely in the database.

     - **User Login API (POST)**
       - Authenticates users by checking their username and password.
       - Returns an authentication token after a successful login.
       - Requires all APIs (except registration) to check this token.

     - **Ticket Booking API (POST)**
       - Enables booking of tickets, including connecting flights (multiple flights in one API call).
       - Uses optimistic locking with SQLx to ensure data consistency by verifying the database state before finalizing transactions.
       - Rolls back transactions if inconsistencies are found due to concurrent actions.

     - **Seat Reservation API (POST)**
       - Allows users to reserve or modify their seat choice on a flight.
       - Manages concurrent seat reservations with SQLx using optimistic locking.
       - Checks data consistency before finalizing transactions, rolling back if necessary.

     - **Available Flights API (GET)**
       - Returns a list of available flights based on location, destination, and date.
       - Responds with flight details in JSON format.

     - **Available Seats API (GET)**
       - Provides information on booked and available seats for a specific flight.

     - **Booking History API**
       - Allows users get their current booked flight information.

   - **Optional Features (Time Permitting)**
     - **Flight Creation API (POST)**
       - Allows new flights to be added to the database with a POST request.
       - Accepts flight details in JSON format.

     - **Flight Cancellation API (DELETE)**
       - Cancels flights via a DELETE request.
       - Changes the flight's status to "canceled" in the database.

     - **Flight Update API (PUT)**
       - Allows updates to flight details using a PUT request.

2. **Database Implementation**
   - Design a MySQL database with essential tables (tickets, flights, seats).
   - Establish table relationships that match the airline booking domain.
   - Ensure the database schema supports all operations while maintaining data integrity.

3. **Demonstration Scripts**
   - Create Bash or Python scripts to test and demonstrate the backend server.
   - Use scripts to simulate API requests and show responses.
   - Provide sample scripts for ticket booking, seat reservations, and other key tasks.

---

### Expected Outcome

By the project’s end, users should be able to:

- Register and securely log into the airline booking system.
- Search for flights based on certain criteria.
- Book tickets and reserve seats with data consistency, even under high concurrency.
- Interact with a backend that handles requests efficiently and keeps performance high.

---

## Tentative plan

As of the writing of this proposal, there are approximately six weeks until the project is due for delivery. The implementation of this project is parallelizable, so each of the two team members can work in parallel to implement the key features of the airline booking system. The implementation is divided into four segments, with segments 1 and 4 taking approximately one week each, and segments 2 and 3 taking approximately two weeks each due to their complexities.

1. **First segment**  
    In the first segment, the team will implement the basic supporting infrastructure for the system. As Guanhong has previous experience with MySQL databases, he will first set up the local MySQL database environment for local testing, and design the database schema for the system. He will then share the setup instructions with Shengxiang to avoid duplicated efforts. The database schema will be kept as simple as possible, but still sufficient to demonstrate the system's performance implemented in Rust. In the meantime, Shengxiang will explore the Rust SQLx crates and implement the database interfacing infrastructure in Rust. These two parts can then be connected together to form the database backend of the airline ticket booking system.

2. **Second segment**  
    In the second segment, the team will implement the basic APIs for the backend server in Rust. Guanhong will explore the Rocket and Actix Web crates and implement the API for a user to create an account, log in and query ticket booking histories. If time permits, he will also implement basic admin functions to the system, including APIs for adding the flights available for booking. With the assumption that a user is logged in and flights are available for booking, Shengxiang will implement the basic ticket booking and seat reservation API to the system. This includes querying and displaying available seats on a particular flight, booking a single user-specified seat selection, as well as auto-assigning the first available seat if the user chooses not to specify seats.

3. **Third segment**  
    In the third segment, the team will build up the basic APIs with more complex and performance-related features. Guanhong will enhance the ticket booking APIs to enable more complex flight bookings, including booking for multiple passengers, selecting multiple seats, and booking connecting flights. Shengxiang will implement the optimistic locking mechanism with the Rust SQLx crates to avoid double selling a seat when the requests are highly concurrent. This will include checking data consistency before finalizing the booking, and rolling back if the seat is no longer available.

4. **Fourth segment**  
    In the fourth segment, the team will focus on testing and ensuring the performance of the system, and time is reserved for fixing the performance issues if it is not satisfying. The team plans to build Bash or Python scripts to issue concurrent booking requests to the backend server, and demonstrate that the system can handle highly concurrent requests with no double selling of seats. Guanhong will build such testing scripts, and in the meantime, Shengxiang will work on the final deliverables of the project, including the reproduction documentation and the video demo.

In summary, the team segmented the project into four milestones. Each milestone is estimated to take approximately one to two weeks depending on their complexities. With the final project delivery date about six weeks away, the team is confident that the project goal can be achieved and delivered.

> ### Reference
> [1] BUREAU OF TRANSPORTATION STATISTICS, “Passengers Traffic,” Bts.gov, https://www.transtats.bts.gov/Data_Elements.aspx?Data=1 (accessed Nov. 3, 2024).
