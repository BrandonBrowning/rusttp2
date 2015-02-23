
use std::str::from_utf8;

use std::old_io as io;
use std::old_io::{Acceptor, Listener};
use std::old_io::{TcpListener, TcpStream};
use std::old_io::BufferedStream;
use std::old_io::File;
use std::old_io::println;
use std::os;

use self::Method::*;

#[derive(PartialEq, Debug)]
enum Method {
    GET,
}

// TODO: Find a trait for parsing &[u8] instead of &str?
fn parse_method(input: &[u8]) -> Option<Method> {
    match input {
        b"GET" => Some(GET),
        _      => None,
    }
}

#[derive(PartialEq, Debug)]
struct Request<'a> {
    method: Method,
    path: &'a str,
    version: &'a str,
}

impl<'a> Request<'a> {
    fn new(method: Method, path: &'a str, version: &'a str) -> Request<'a> {
        Request {
            method: method,
            path: path,
            version: version,
        }
    }
}

fn parse_http1(input: &[u8]) -> Request {
    let tokens = &mut input.split(|c| *c == b' ');

    let method_raw = tokens.next().unwrap();
    let method = parse_method(method_raw).unwrap();

    let path_raw = tokens.next().unwrap();
    let path = from_utf8(path_raw).unwrap();

    let http_prefix = "HTTP/";
    let version_full_raw = tokens.next().unwrap();
    let version_full = from_utf8(version_full_raw).unwrap();

    let is_beginning_http = version_full.starts_with(http_prefix);
    assert!(is_beginning_http);

    let start_index = http_prefix.len();
    let version = &version_full[start_index..];

    Request::new(method, path, version)

}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:80");
    let mut acceptor = listener.listen();

    println("Listening for connections");
    println("");

    for stream_opt in acceptor.incoming() {
        let stream = stream_opt.unwrap();
        let bstream = &mut BufferedStream::new(stream);

        let mut last_empty = false;
        let mut answered = false;
        loop {
            let line = bstream.read_line().unwrap();

            let line_is_empty = line.len() <= 2;

            if line_is_empty {
                if last_empty {
                    answered = false;
                    last_empty = true;
                }
            } else if !answered {
                let request_output = format!("{}{}", "< ", line.replace("\r\n", "\r\n< "));
                println(request_output.as_slice());

                let request = parse_http1(line.as_bytes());
                
                let path = Path::new(request.path);
                let absolute_path = os::make_absolute(&path).unwrap();

                println!("looking for file at {}", absolute_path.display());

                match File::open(&absolute_path) {
                    Ok(ref mut file) => {
                        let contents = file.read_to_string().unwrap();
                        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n", contents.len(), contents);

                        let response_output = format!("{}{}", "> ", response.replace("\r\n", "\r\n> "));
                        println("");
                        println(response_output.as_slice());

                        bstream.write_all(response.as_bytes());
                    }
                    Err(err) => {
                        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";

                        let response_output = format!("{}{}", "> ", response.replace("\r\n", "\r\n> "));
                        println("");
                        println(response_output.as_slice());

                        bstream.write_all(response.as_bytes());
                    }
                }

                break;
            }
        }
    }

    drop(acceptor);
}

#[test]
fn test_get_root() {
    let request_raw = "GET / HTTP/1.1".as_bytes();
    let request = parse_http1(request_raw);
    let expect = Request::new(GET, "/", "1.1");

    assert!(request == expect);
}

#[test]
fn test_get_directory() {
    let request_raw = "GET /foo HTTP/1.1".as_bytes();
    let request = parse_http1(request_raw);
    let expect = Request::new(GET, "/foo", "1.1");

    assert!(request == expect);
}

#[test]
fn test_get_file() {
    let request_raw = "GET /foo.json HTTP/1.1".as_bytes();
    let request = parse_http1(request_raw);
    let expect = Request::new(GET, "/foo.json", "1.1");

    assert!(request == expect);
}

#[test]
fn test_get_directory_file() {
    let request_raw = "GET /foo/bar.json HTTP/1.1".as_bytes();
    let request = parse_http1(request_raw);
    let expect = Request::new(GET, "/foo/bar.json", "1.1");

    assert!(request == expect);
}

#[test]
fn test_http_10_version() {
    let request_raw = "GET / HTTP/1.0".as_bytes();
    let request = parse_http1(request_raw);

    assert!(request.version == "1.0");
}