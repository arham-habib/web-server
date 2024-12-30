use std::{
    env,
    fs,
    path::PathBuf,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

use web_server::ThreadPool;

fn main() {
    println!("Starting the server");

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

    println!("Received request line: {}", request_line);

    let base_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/app"));
    let (status_line, filename, content_type) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", base_dir.join("html/home.html"), "text/html"),
        "GET /blog HTTP/1.1" => ("HTTP/1.1 200 OK", base_dir.join("html/blog.html"), "text/html"),
        _ if request_line.starts_with("GET /static/") => {
            let path = request_line.split_whitespace().nth(1).unwrap_or("");
            println!("Serving static file: {}", path);
            let file_path = base_dir.join(&path[1..]); // Remove the leading '/'
            let content_type = if file_path.extension().map_or(false, |ext| ext == "css") {
                "text/css"
            } else if file_path.extension().map_or(false, |ext| ext == "js") {
                "application/javascript"
            } else {
                "text/plain"
            };
            ("HTTP/1.1 200 OK", file_path, content_type)
        }
        _ if request_line.starts_with("GET /blog/") => {
            // Extract the path after /blog/ for dynamic handling
            let sub_path = request_line.trim_start_matches("GET /blog/").trim_end_matches(" HTTP/1.1");
            let file_path = base_dir.join("blog").join(sub_path);

            println!("{}", file_path.display());
            
            if file_path.exists() && file_path.is_file() {
                ("HTTP/1.1 200 OK", file_path, "text/html") // Adjust content type as needed
            } else {
                ("HTTP/1.1 404 NOT FOUND", base_dir.join("html/404.html"), "text/html")
            }
        }
        _ => ("HTTP/1.1 404 NOT FOUND", base_dir.join("html/404.html"), "text/html"),
    };

    // Attempt to read the file
    if let Ok(contents) = fs::read_to_string(&filename) {
        let length = contents.len();
        let response = format!(
            "{status_line}\r\nContent-Length: {length}\r\nContent-Type: {content_type}\r\n\r\n{contents}"
        );
        stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Failed to write response: {}", e); // Log response write failures
        }); // Minimum edit: Log errors while writing responses
    } else {
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Failed to write 404 response: {}", e); // Log 404 response write failures
        });
    }
}
