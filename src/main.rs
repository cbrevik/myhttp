#![feature(bind_by_move_pattern_guards)]

use myhttp::ThreadPool;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

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

    match stream.read(&mut buffer) {
        Ok(bytes) => println!("Read {} bytes from request stream", bytes),
        Err(e) => eprintln!("Error when reading from request stream: {}", e),
    }

    let request = String::from_utf8_lossy(&buffer[..]);

    let response = match get_response(String::from(request)) {
        Ok(response) => response,
        Err(err) => get_500_response(err),
    };

    match stream.write(&response) {
        Ok(bytes) => println!("Wrote {} bytes to response stream", bytes),
        Err(e) => eprintln!("Error when writing to response stream: {}", e),
    };
    stream.flush().unwrap();
}

fn get_response(request: String) -> Result<Vec<u8>, Box<dyn Error>> {
    let path = get_path_from_request(request)?;

    if path.exists() {
        let metadata = path.metadata()?;

        if metadata.is_dir() {
            let dir = path.read_dir()?;

            match find_index_file(dir) {
                Some(index_file) => {
                    let contents = fs::read_to_string(index_file)?;
                    Ok(get_200_response(Vec::from(contents.as_bytes())))
                }
                None => Ok(get_dir_response(path.read_dir()?)),
            }
        } else {
            let contents = fs::read(path)?;
            Ok(get_200_response(contents))
        }
    } else {
        Ok(get_404_response())
    }
}

fn get_path_from_request(request: String) -> Result<PathBuf, &'static str> {
    match request.lines().nth(0) {
        Some(res) => {
            let mut splits = res.split(' ');
            match splits.nth(1) {
                Some(url) => {
                    let formatted = format!(".{}", url);
                    Ok(Path::new(&formatted).to_owned())
                }
                None => Err("Couldn't find the URL part of the request"),
            }
        }
        None => Err("Request was empty."),
    }
}

fn find_index_file(entries: fs::ReadDir) -> Option<PathBuf> {
    for entry in entries {
        if let Ok(e) = entry {
            let file_name = e.file_name();
            if file_name == "index.html" || file_name == "index.htm" {
                return Some(e.path());
            }
        }
    }

    None
}

fn get_200_response(mut contents: Vec<u8>) -> Vec<u8> {
    //let response = format!("{}{}", "HTTP/1.1 200 OK\r\n\r\n", contents);
    let mut response = Vec::from("HTTP/1.1 200 OK\r\n\r\n".as_bytes());
    response.append(&mut contents);
    response
}

fn get_404_response() -> Vec<u8> {
    Vec::from(String::from("HTTP/1.1 404 NOT FOUND\r\n\r\n404 Not Found").as_bytes())
}

fn get_500_response(error: Box<dyn Error>) -> Vec<u8> {
    let response = format!(
        "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n500 Internal Server Error: {}",
        error
    );

    Vec::from(response.as_bytes())
}

fn get_dir_response(entries: fs::ReadDir) -> Vec<u8> {
    let entry_html = entries
        .map(|e| e.unwrap())
        .fold(String::new(), |prev, dir_entry| {
            let path = dir_entry.path();
            prev + &format!(
                "<li><a href=\"/{}\">{}</a></li>",
                path.strip_prefix(".").unwrap().to_str().unwrap(),
                path.file_name().unwrap().to_str().unwrap()
            )
        });
    Vec::from(
        format!(
            "HTTP/1.1 200 OK\r\n\r\n<html><body><ul>{}</ul></body></html>",
            entry_html
        )
        .as_bytes(),
    )
}
