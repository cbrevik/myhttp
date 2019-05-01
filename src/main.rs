use myhttp::ThreadPool;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use typed_html::elements::{li, FlowContent};
use typed_html::types::Uri;

use typed_html::{html, text, OutputType};

#[derive(StructOpt)]
struct Cli {
    /// The local host port
    #[structopt(short = "p", long = "port")]
    port: Option<u16>,
}

fn main() {
    let opt = Cli::from_args();

    let port = match opt.port {
        Some(port) => port,
        None => 8080,
    };
    let host = format!("{}:{}", "127.0.0.1", port);

    let listener = TcpListener::bind(&host).unwrap();
    let pool = ThreadPool::new(4);

    println!("Listening to host: http://{}", &host);

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
                    let contents = fs::read(index_file)?;
                    Ok(get_200_response(contents))
                }
                None => Ok(get_dir_response(path.to_str().unwrap(), path.read_dir()?)),
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
    let mut response = Vec::from("HTTP/1.1 200 OK\r\n\r\n".as_bytes());
    response.append(&mut contents);
    response
}

fn get_basic_response_body<T: OutputType + 'static>(
    title: &str,
    content: Box<dyn FlowContent<T>>,
) -> String {
    html!(
        <html>
            <head>
                <title>{ text!(title) }</title>
            </head>
            <body>
                {content}
            </body>
        </html>
    )
    .to_string()
}

fn get_basic_http_response(response_code: &str, response_body: String) -> Vec<u8> {
    Vec::from(format!("HTTP/1.1 {}\r\n\r\n{}", response_code, response_body).as_bytes())
}

fn get_404_response() -> Vec<u8> {
    let doc = get_basic_response_body::<String>("Not Found", html!(<p>"404 Not Found"</p>));
    get_basic_http_response("404 NOT FOUND", doc)
}

fn get_500_response(error: Box<dyn Error>) -> Vec<u8> {
    let doc = get_basic_response_body::<String>(
        "Internal Server Error",
        html!(<p>{text!(format!("500 Internal Server Error: {}", error))}</p>),
    );

    get_basic_http_response("500 INTERNAL SERVER ERROR", doc.to_string())
}

fn get_dir_response(dir_path: &str, entries: fs::ReadDir) -> Vec<u8> {
    let wrap_li = |href: Uri, name: &str| {
        let element: Box<li<_>> = html!(
            <li><a href={href}>{ text!(name) }</a></li>
        );
        element
    };
    let li_elements: Vec<Box<li<_>>> = entries
        .map(std::result::Result::unwrap)
        .map(|dir_entry| {
            let path = dir_entry.path();
            wrap_li(
                String::from(path.strip_prefix(".").unwrap().to_str().unwrap()),
                path.file_name().unwrap().to_str().unwrap(),
            )
        })
        .collect();
    let doc = get_basic_response_body::<String>(
        dir_path,
        html!(
            <ul>{li_elements}</ul>
        ),
    );
    get_basic_http_response("200 OK", doc.to_string())
}
