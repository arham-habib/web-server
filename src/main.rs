use std::{
    env,
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

use web_server::ThreadPool;

fn main() {
    // determine the port in the environment variable, or default to 7878
    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());

    // determine the host: in production, bind to 0.0.0.0; in development, bind to 127.0.0.1
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let address = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&address).unwrap_or_else(|err| {
        eprintln!("Failed to bind to address {}: {}", address, err);
        std::process::exit(1);
    });
    
    println!("Server is listening on {}", address);

    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s, 
            Err(e) => {
                eprintln!("Connection failed: {}", e);
                continue;
            }
        };

        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = match buf_reader.lines().next() {
        Some(Ok(line)) => line,
        _ => {
            eprintln!("Failed to read request line");
            return;
        }
    };
    
    let (_status_line, filename, content_type) = if request_line.starts_with("GET /static/") {
        let path = &request_line[5..].split_whitespace().next().unwrap();
        println!("serving static files, {}", path);
        let filename = format!("./{}", path);
        
        let content_type = if filename.ends_with(".css") {
            "text/css"
        } else {
            "text/plain"
        };
        
        ("HTTP/1.1 200 OK", filename, content_type)
    } else {
        // Handle other routes
        let (status_line, file_path) = match &request_line[..] {
            "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "html/home.html"),
            "GET /blog HTTP/1.1" => ("HTTP/1.1 200 OK", "html/blog.html"),
            _ => ("HTTP/1.1 404 NOT FOUND", "html/404.html"),
        };
        
        ("HTTP/1.1 200 OK", file_path.to_string(), "text/html")
    };

    // Attempt to read the file
    if let Ok(contents) = fs::read_to_string(&filename) {
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\nContent-Type: {content_type}\r\n\r\n{contents}");
        stream.write_all(response.as_bytes()).unwrap();
    } else {
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    }
}
