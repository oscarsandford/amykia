use std::{
	io::{Read, BufReader, Write},
	net::TcpStream,
	fs,
};

pub mod net;

const RECV_BUFFER_SIZE: usize = 1024;
const PUBLIC_PFX: &str = "./public";

#[derive(Debug)]
enum AKErr<'a> {
	ParseError(std::str::Utf8Error),
	HeaderError(&'a str),
}
impl std::error::Error for AKErr<'_> {}
impl std::fmt::Display for AKErr<'_>  {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ParseError(e) => write!(f, "[!] Error parsing the request body: {}", e),
			Self::HeaderError(e) => write!(f, "[!] Error parsing the HTTP header: {}", e),
		}
	}
}
impl From<std::str::Utf8Error> for AKErr<'_>  {
	fn from(e: std::str::Utf8Error) -> Self { Self::ParseError(e) }
}


// All resources can be returned in bytes form to send back.
trait Resource {
	fn get_html(&self) -> String;
	fn get_path(&self) -> &str;
	fn can_download(&self) -> bool;
}

struct File {
	path: String, // Should be &str, but we'll mess with this lifetime later.
	html: String,
	dl: bool, // Can download? (Maybe instead this field should be "can view")
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
	accept: Vec<&'a str>,
}

struct Response<'a, 'b> {
	protocol: &'a str,
	code: u16,
	status: &'b str,
	content: Vec<u8>,
}


/// Return a simple HTML page for display an error code, status, and message.
fn fmt_err_page(code: u16, status: &str, e: String) -> Vec<u8> {
	format!("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"/><title>Error {code} {status}</title></head><h1>Error {code}: {status}</h1><body>{e}</body></html>")
	.into_bytes().to_vec()
}

/// Parse a HTTP request header from a given bytes buffer `buf`.
fn parse(buf: &[u8]) -> Result<Request, AKErr> {
	let buf_str = std::str::from_utf8(buf)?;
	let lines = buf_str.split("\r\n").collect::<Vec<&str>>();
	let mut first = lines.first().unwrap_or(&"").splitn(3, ' ');
	let method = first.next().ok_or(AKErr::HeaderError("HTTP method unretrievable"))?;
	let route = first.next().ok_or(AKErr::HeaderError("HTTP route unretrievable"))?;
	let protocol = first.next().ok_or(AKErr::HeaderError("HTTP protocol unretrievable"))?;
	// TODO: Might not need the accept fields...
	let accept = lines.iter()
		.find(|l| l.starts_with("Accept: "))
		.map(|l| &l[8..])
		.unwrap_or(&"")
		.split(',')
		.collect();
	Ok( Request { method, route, protocol, accept } )
}

/// Join and return a response header and its content in a bytes vector.
fn package(res: Response) -> Vec<u8> {
	[format!("{} {} {}\r\nContent-Length: {}\r\n\r\n", 
		res.protocol, res.code, res.status, res.content.len()).into_bytes(),
	res.content].concat()
}

/// Handle a given bytes buffer `buf` by parsing it. If successful, 
/// determine a local resource from which to construct a response package.
fn handle(buf: &[u8]) -> Vec<u8> {
	match parse(&buf) {
		Err(e) => package(Response { protocol: "HTTP/1.1", code: 400, status: "BAD REQUEST", content: fmt_err_page(400, "BAD REQUEST", e.to_string()) }),
		Ok(req) => {
			dbg!(&req);
			let resource_path = match req.route {
				"/" => format!("{}{}", PUBLIC_PFX, "/index.html"),
				route => format!("{}{}", PUBLIC_PFX, route),
			};
			
			// Read as bytes so that we can return downloadable media as well as HTML files.
			// FYI: PDFs get embeded, but images get downloaded right away (not embeded).
			match fs::read(resource_path) {
				Ok(resource) => package(Response { protocol: "HTTP/1.1", code: 200, status: "OK", content: resource }),
				Err(e) => {
					eprintln!("404: {:?}", e);
					package(Response { protocol: "HTTP/1.1", code: 404, status: "NOT FOUND", content: fmt_err_page(404, "NOT FOUND", e.to_string()) })
				},
			}
		},
	}
}

/// Receives a `TcpStream` by reading into a fixed size buffer, 
/// calling a function to handle creating a response of an 
/// indeterminate number of bytes, and then writing them to 
/// the stream.
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
}

#[cfg(test)]
mod tests {
    use crate::*;

	#[test]
	fn parse_simple_buffer() {
		let buf = "GET / HTTP/1.1".as_bytes();
		let req = parse(&buf).unwrap();
		assert_eq!(req, Request { method: "GET", route: "/", protocol: "HTTP/1.1", accept: vec![""] });
	}

	#[test]
	fn parse_empty_buffer() {
		let buf = "".as_bytes();
		parse(&buf).expect_err("Success!");
	}
}