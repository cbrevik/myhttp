#![feature(bind_by_move_pattern_guards)]

use myhttp::ThreadPool;
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

    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);

    let response = get_response(String::from(request));

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn get_response(request: String) -> String {
    match get_path_from_request(request) {
        Some(path) if path.exists() => match path.metadata() {
            Ok(metadata) if metadata.is_dir() => match path.read_dir() {
                Ok(dir) => match find_index_file(dir) {
                    Some(index_file) => match fs::read_to_string(index_file) {
                        Ok(contents) => get_200_response(contents),
                        _ => get_401_response(),
                    },
                    None => get_dir_response(path.read_dir().unwrap()),
                },
                Err(_) => get_401_response(),
            },
            Ok(metadata) if metadata.is_file() => match fs::read_to_string(path) {
                Ok(contents) => get_200_response(contents),
                _ => get_401_response(),
            },
            _ => get_401_response(),
        },
        _ => get_404_response(),
    }
}

fn get_path_from_request(request: String) -> Option<PathBuf> {
    if let Some(res) = request.lines().nth(0) {
        let mut splits = res.split(" ");
        if let Some(url) = splits.nth(1) {
            let formatted = format!(".{}", url);
            let path = Path::new(&formatted);
            return Some(path.to_owned());
        }
    }

    None
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

fn get_dir_response(entries: fs::ReadDir) -> String {
    let entry_html = entries
        .map(|e| e.unwrap())
        .fold(String::new(), |prev, dir_entry| {
            let path = dir_entry.path();
            let result = prev
                + &format!(
                    "<li><a href=\"/{}\">{}</a></li>",
                    path.strip_prefix(".").unwrap().to_str().unwrap(),
                    path.file_name().unwrap().to_str().unwrap()
                );
            result
        });
    format!(
        "HTTP/1.1 200 OK\r\n\r\n<html><body><ul>{}</ul></body></html>",
        entry_html
    )
}
