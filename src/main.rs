use myhttp::ThreadPool;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];

    stream.read(&mut buffer).unwrap();

    let mut response = get_404_response();

    let request = String::from_utf8_lossy(&buffer[..]);

    let mut lines = request.lines();
    if let Some(res) = lines.next() {
        let mut splits = res.split(" ");
        splits.next();
        if let Some(url) = splits.next() {
            let formatted_path = format!(".{}", url);
            let path = Path::new(&formatted_path);
            if path.exists() {
                if let Ok(metadata) = path.metadata() {
                    if metadata.is_dir() {
                        if let Ok(mut dir) = path.read_dir() {
                            let has_index = dir.any(|entry| {
                                if let Ok(e) = entry {
                                    e.file_name() == "index.html"
                                } else {
                                    false
                                }
                            });

                            if has_index {
                                let file_path = format!("{}/index.html", formatted_path);
                                let contents = fs::read_to_string(file_path).unwrap();
                                response = get_200_response(contents);
                            } else {
                                response = get_200_response(get_dir_html(formatted_path));
                            }
                        } else {
                            response = get_401_response();
                        }
                    } else {
                        let contents = fs::read_to_string(formatted_path).unwrap();
                        response = get_200_response(contents);
                    }
                }
            }
        }
    }

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn get_200_response(contents: String) -> String {
    let response = format!("{}{}", "HTTP/1.1 200 OK\r\n\r\n", contents);
    response
}

fn get_401_response() -> String {
    String::from("HTTP/1.1 401 UNAUTHORIZED\r\n\r\n401 Unauthorized")
}

fn get_404_response() -> String {
    String::from("HTTP/1.1 404 NOT FOUND\r\n\r\n404 Not Found")
}

fn get_dir_html(path: String) -> String {
    for entry in fs::read_dir(path) {
        println!("{:?}", entry);
    }
    format!("<html>{}</html>", "0")
}
