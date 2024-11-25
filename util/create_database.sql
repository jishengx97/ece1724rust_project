create database airline_reservation_system;
use airline_reservation_system;

-- Table aircraft
create table aircraft
(
    aircraft_id int not null
        primary key,
    capacity    int not null
);

-- Default Aircraft
INSERT INTO aircraft (aircraft_id, capacity)
VALUES (737, 169);

INSERT INTO aircraft (aircraft_id, capacity)
VALUES (777, 400);

INSERT INTO aircraft (aircraft_id, capacity)
VALUES (320, 146);


INSERT INTO aircraft (aircraft_id, capacity)
VALUES (900, 76);

INSERT INTO aircraft (aircraft_id, capacity)
VALUES (200, 50);


-- Table: User
create table user
(
    id       int auto_increment
        primary key,
    username char(255)                             not null,
    password char(255)                             not null,
    role     enum ('ADMIN', 'USER') default 'USER' not null,
    constraint user_username_uindex
        unique (username)
);


-- Table Customer Info
create table customer_info
(
    id         int                     not null
        primary key,
    name       char(255)               not null,
    birth_date date                    not null,
    gender     enum ('male', 'female') not null,
    constraint customer_info_user_id_fk
        foreign key (id) references user (id)
            on delete cascade
);


-- Table flightRoute route
create table flight_route
(
    flight_number    int                        not null
        primary key,
    departure_city   char(255)                  not null,
    destination_city char(255)                  not null,
    departure_time   time                       not null,
    arrival_time     time                       not null,
    aircraft_id      int                        not null,
    overbooking      decimal(4, 2) default 0.00 not null,
    start_date       date                       not null,
    end_date         date                       null,
    constraint flight_route_aircraft_aircraft_id_fk
        foreign key (aircraft_id) references aircraft (aircraft_id)
            on update cascade on delete cascade
);


-- Table flight
create table flight
(
    flight_id         int auto_increment
        primary key,
    flight_number     int  not null,
    flight_date       date not null,
    available_tickets int  not null,
    version           int  null,
    constraint flight_flight_route_flight_number_fk
        foreign key (flight_number) references flight_route (flight_number)
            on update cascade on delete cascade
);



-- Table flight seat info
create table seat_info
(
    flight_id   int                                                          not null,
    seat_number int                                                          not null,
    seat_status enum ('AVAILABLE', 'UNAVAILABLE', 'BOOKED') default 'BOOKED' not null,
    constraint seat_info_flight_id_seat_number_uindex
        unique (flight_id, seat_number),
    constraint seat_info_flight_flight_id_fk
        foreign key (flight_id) references flight (flight_id)
            on delete cascade
);

alter table seat_info
    add primary key (flight_id, seat_number);


-- Table ticket
create table ticket
(
    id            int auto_increment
        primary key,
    customer_id   int  not null,
    flight_id     int  not null,
    seat_number   int  null,
    flight_date   date not null,
    flight_number int  not null,
    constraint ticket_customer_info_id_fk
        foreign key (customer_id) references customer_info (id)
            on delete cascade,
    constraint ticket_flight_id_fk
        foreign key (flight_id) references flight (flight_id)
            on delete cascade,
    constraint ticket_seat_info_flight_id_seat_number_fk
        foreign key (flight_id, seat_number) references seat_info (flight_id, seat_number)
);