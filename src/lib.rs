use std::{
	io::{Read, BufReader, Write},
	net::TcpStream,
	fs,
};

pub mod net;

const RECV_BUFFER_SIZE: usize = 1024;
const PUBLIC_PFX: &str = "./public";
const HTML404: &str = "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"/><title>Error 404</title></head><body><h1>404 NOT FOUND</h1></body></html>";


// All resources can be returned in bytes form to send back.
trait Resource {
	fn get_html(&self) -> String;
	fn get_path(&self) -> &str;
	fn can_download(&self) -> bool;
}

struct File {
	path: String, // Should be &str, but we'll mess with this lifetime later.
	html: String,
	dl: bool, // Can download?
}

impl Resource for File {
	fn get_html(&self) -> String { String::from("<p>hello</p>") }
	fn get_path(&self) -> &str { "" }
	fn can_download(&self) -> bool { self.dl }
}

#[derive(Debug, PartialEq)]
struct Request<'a> {
	method: &'a str,
	route: &'a str,
	protocol: &'a str,
	accepts: Vec<&'a str>,
}

struct Response<'a, 'b> {
	protocol: &'a str,
	code: u16,
	status: &'b str,
	content: String,
}

fn parse(buf: &[u8]) -> Request  {
	let buf_str = std::str::from_utf8(buf).unwrap();
	let lines = buf_str.split("\r\n").collect::<Vec<&str>>();
	let mut first = lines.first().unwrap_or(&"").splitn(3, ' ');
	let method = first.next().unwrap_or_default();
	let route = first.next().unwrap_or_default();
	let protocol = first.next().unwrap_or_default();
	let accepts = lines.get(6).unwrap_or(&"")
				.splitn(1, ':').next().unwrap_or(&"")
				.split(',').collect::<Vec<&str>>();
	// TODO: deal with bad requests
	Request { method, route, protocol, accepts }
}

fn package(res: Response) -> Vec<u8> {
	format!("{} {} {}\r\nContent-Length: {}\r\n\r\n{}", 
		res.protocol, res.code, res.status, res.content.len(), res.content)
		.into_bytes()
}

fn handle(buf: &[u8]) -> Vec<u8> {
	// Parse the request. Where are we going?
	let req = parse(&buf);
	println!("{req:?}");
	let resource_path = match req.route {
		"/" => format!("{}{}", PUBLIC_PFX, "/index.html"),
		route => format!("{}{}", PUBLIC_PFX, route),
	};
	match fs::read_to_string(resource_path) {
		Ok(resource) => package(Response { protocol: "HTTP/1.1", code: 200, status: "OK", content: resource }),
		Err(_) => package(Response { protocol: "HTTP/1.1", code: 404, status: "NOT FOUND", content: HTML404.to_string() }),
	}
}

pub fn receive(mut stream: TcpStream) {
	let mut buf_reader = BufReader::new(&mut stream);
	let mut buf = [0u8; RECV_BUFFER_SIZE];
	if let Err(e) = buf_reader.read(&mut buf) {
		eprintln!("Socket read failed: {:?}", e);
	};
	let res = handle(&buf);
	if let Err(e) = stream.write_all(&res) {
		eprintln!("Socket write failed: {:?}", e);
	}
	// TODO: do we want a stream.flush() here?
}

#[cfg(test)]
mod tests {
    use crate::*;

	#[test]
	fn handle_empty_buffer() {
		let buf = [0u8; RECV_BUFFER_SIZE];
		let res = handle(&buf);
		assert_eq!(res, package(Response { protocol: "HTTP/1.1", code: 404, status: "NOT FOUND", content: HTML404.to_string() }));
	}

	#[test]
	fn parse_simple_buffer() {
		let buf = "GET / HTTP/1.1".as_bytes();
		let req = parse(&buf);
		assert_eq!(req, Request { method: "GET", route: "/", protocol: "HTTP/1.1", accepts: vec![""] });
	}
}