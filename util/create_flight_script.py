import mysql.connector
from datetime import datetime, timedelta
import math
from dotenv import load_dotenv
import os
from urllib.parse import urlparse

def connect_to_db():
    load_dotenv()
    
    db_url = urlparse(os.getenv('DATABASE_URL'))
    
    return mysql.connector.connect(
        host=db_url.hostname,
        user=db_url.username,
        password=db_url.password,
        database=db_url.path[1:]  # Remove '/'
    )

def add_flight_route_and_flights(
    flight_number,
    departure_city,
    destination_city,
    departure_time,
    arrival_time,
    aircraft_id,
    overbooking,
    start_date,
    end_date
):
    try:
        conn = connect_to_db()
        cursor = conn.cursor(dictionary=True)
        
        # 1. Get aircraft capacity
        cursor.execute("SELECT capacity FROM aircraft WHERE aircraft_id = %s", (aircraft_id,))
        aircraft_capacity = cursor.fetchone()['capacity']
        
        # 2. Calculate available tickets considering overbooking
        available_tickets = math.ceil(aircraft_capacity * (1 + overbooking))
        
        # 3. Insert route information
        insert_route_sql = """
        INSERT INTO flight_route 
        (flight_number, departure_city, destination_city, departure_time, arrival_time, 
         aircraft_id, overbooking, start_date, end_date)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
        """
        route_data = (flight_number, departure_city, destination_city, departure_time, 
                     arrival_time, aircraft_id, overbooking, start_date, end_date)
        cursor.execute(insert_route_sql, route_data)
        
        # 4. Create flights for each day
        current_date = datetime.strptime(start_date, '%Y-%m-%d')
        end_date = datetime.strptime(end_date, '%Y-%m-%d')
        
        print(f"Start creating flights from {start_date} to {end_date}")
        print(f"Aircraft capacity: {aircraft_capacity}, Available tickets: {available_tickets}")
        
        while current_date <= end_date:
            try:
                # 5. Insert flight information
                insert_flight_sql = """
                INSERT INTO flight (flight_number, flight_date, available_tickets, version)
                VALUES (%s, %s, %s, 1)
                """
                flight_data = (flight_number, current_date.strftime('%Y-%m-%d'), available_tickets)
                # print(f"Creating flight: {flight_data}")
                cursor.execute(insert_flight_sql, flight_data)
                
                # 6. Get the flight_id of the recently inserted flight
                flight_id = cursor.lastrowid
                # print(f"Created Flight ID: {flight_id}")
                
                # 7. Create seat information for each seat
                seat_values = [(flight_id, seat_num, 'AVAILABLE') 
                              for seat_num in range(1, aircraft_capacity + 1)]
                
                print(f"Create {len(seat_values)} seats for flight {flight_id}")
                insert_seats_sql = """
                INSERT INTO unavailable_seat_info 
                (flight_id, seat_number, seat_status)
                VALUES (%s, %s, %s)
                """
                cursor.executemany(insert_seats_sql, seat_values)
                # print(f"Seats created")
                
                current_date += timedelta(days=1)
            
            except mysql.connector.Error as err:
                print(f"Error processing date {current_date}: {err}")
                raise  # Re-throw
        
        conn.commit()
        print(f"Successfully added flight route and flights for {flight_number}")
        
    except mysql.connector.Error as err:
        print(f"Error: {err}")
        conn.rollback()
    finally:
        cursor.close()
        conn.close()

if __name__ == "__main__":
    flight_routes = [
        {
            "flight_number": 590,
            "departure_city": "IAH",
            "destination_city": "YYZ",
            "departure_time": "07:20:00",
            "arrival_time": "11:26:00",
            "aircraft_id": 320,
            "overbooking": 0.015,
            "start_date": "2024-10-24",
            "end_date": "2024-11-10"
        },
        {
            "flight_number": 1284,
            "departure_city": "LAS",
            "destination_city": "YYZ",
            "departure_time": "23:55:00",
            "arrival_time": "07:00:00",
            "aircraft_id": 737,
            "overbooking": 0.03,
            "start_date": "2024-10-24",
            "end_date": "2024-11-10"
        }
    ]

    for route in flight_routes:
        add_flight_route_and_flights(**route)