use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[macro_use]
extern crate serde_derive;

// Model User: id, name, email
#[derive(Serialize, Deserialize)]
pub struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

const DB_URL: &str = env!("DATABASE_URL");
const OK_RESP: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SRV_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

// Main
fn main() {
    // Set db
    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }

    // Start server
    let listener = TcpListener::bind(format!(0.0.0.0:8080)).unwrap();
    println!("Server started at port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => println!("Error: {}", e),
        }
    }
}

// Handle client
pub fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (stat_line, content) = match &*request {
                r if request_with("POST /users") => handle_post_request(r),
                r if request_with("GET /users/") => handle_post_request(r),
                r if request_with("GET /users") => handle_post_request(r),
                r if request_with("PUT /users/") => handle_post_request(r),
                r if request_with("DELETE /users") => handle_post_request(r),
                _ => (NOT_FOUND.to_string(), "NOT FOUND".to_string()),
            };
            stream.write_all(format!("{}{}", stat_line, content).as_bytes()).unwrap();
        }
        Err(e) => println!("Error: {}", e),
    }
}

/// TODO Handle GET request
pub fn handle_get_request(req: &str) -> (String, String) {
    match (get_id(&req).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };

                    (OK_RESP.to_string(), serde_json::to_string(&user).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "User not found".to_string()),
            }

        _ => (INTERNAL_SRV_ERROR.to_string(), "Error".to_string()),
    }
}

// Controller

// Handle post req
fn handle_post_request(req: &str) -> (String, String) {
    match (get_user_req_body(&req), Client::connect(DB_URL, NoTls)) {
        (Ok(user), Ok(mut client)) => {
            client.execute(
                "INSERT INTO users (name, email) VALUES ($1, $2)_",
                &[&user.name, &user.email]
            ).unwrap();
            (OK_RESP.to_string(), "User created".to_string())
        }
        _ => (INTERNAL_SRV_ERROR.to_string(), "Error".to_string()),
    }
}

pub fn set_database() -> Result<(), PostgresError> {
    // Connect db
    let mut client = Client::connect(DB_URL, NoTls)?;

    client.execute("CREATE TABLE IF NOT EXISTS user (\
     id SERIAL PRIMARY KEY,\
      name VARCAR NOT NULL,\
       email VARCHAR NOT NULL)",
                   &[])?
}

// Get id
pub fn get_id(req: &str) -> &str {
    req.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

// DE user from request body without id
pub fn get_user_req_body(req: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(req.split("\r\n\r\n").last().unwrap_or_default())
}
