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
// trait Resource {
// 	fn get_html(&self) -> String;
// 	fn get_path(&self) -> &str;
// 	fn can_download(&self) -> bool;
// }

// struct File {
// 	path: String, // Should be &str, but we'll mess with this lifetime later.
// 	html: String,
// 	dl: bool, // Can download? (Maybe instead this field should be "can view")
// }

// impl Resource for File {
// 	fn get_html(&self) -> String { String::from("<p>hello</p>") }
// 	fn get_path(&self) -> &str { "" }
// 	fn can_download(&self) -> bool { self.dl }
// }

#[derive(Debug, PartialEq)]
struct Request<'a> {
	method: &'a str,
	route: &'a str,
	protocol: &'a str,
	// accept: Vec<&'a str>,
}


/// Parse a HTTP request header from a given bytes buffer `buf`.
fn parse(buf: &[u8]) -> Result<Request, AKErr> {
	let buf_str = std::str::from_utf8(buf)?;
	let lines = buf_str.split("\r\n").collect::<Vec<&str>>();
	let mut first = lines.first().unwrap_or(&"").splitn(3, ' ');
	let method = first.next().ok_or(AKErr::HeaderError("HTTP method unretrievable"))?;
	let route = first.next().ok_or(AKErr::HeaderError("HTTP route unretrievable"))?;
	let protocol = first.next().ok_or(AKErr::HeaderError("HTTP protocol unretrievable"))?;
	Ok( Request { method, route, protocol } )
}

/// Return a simple HTML page for display a code, status, and message.
fn err_html(
	code: u16, 
	status: &str, 
	e: String
) -> Vec<u8> {
	format!("<html><head><title>{code} {status}</title></head><h1>{code} {status}</h1><body>{e}</body></html>")
	.into_bytes()
}

/// Create a simple HTML page that serves as a graphical representation of the 
/// directory `dir` with links to navigate to child files and other directories.
fn dir_html(dir: &String) -> std::io::Result<Vec<u8>> {
	let relpath = &dir[PUBLIC_PFX.len()..].trim_end_matches('/');
	let mut html = format!("<html><head><title>{relpath}</title></head><h1>{relpath}</h1><body><ul>").into_bytes();
	for entry in fs::read_dir(dir)? {
		let entry = entry?;
		let fname = entry.file_name();
		let fname = fname.to_string_lossy();
		// if entry.path().is_dir() { }
		html.extend(&format!("<li><a href=\"{relpath}/{fname}\">{fname}<a></li>").into_bytes());
	}
	html.extend(b"</ul></body></html>");
	Ok(html)
}

/// This function is called when there was an error reading the resource as a file.
/// In this case, we try to construct a directory page based on the resource `path` 
/// and package it. If still unsuccessful, we package a 404 page.
fn package_directory<'a>(
	protocol: &'a str, 
	path: String
) -> Vec<u8> {
	match dir_html(&path) {
		Ok(html) => package(protocol, 200, "OK", html),
		Err(e) => package(protocol, 404, "NOT FOUND", err_html(404, "NOT FOUND", e.to_string())),
	}
}

/// Join and return a response header and its content in a bytes vector.
fn package<'a, 'b>(
	protocol: &'a str, 
	code: u16, 
	status: &'b str, 
	content: Vec<u8>
) -> Vec<u8> {
	[format!("{} {} {}\r\nContent-Length: {}\r\n\r\n", 
		protocol, code, status, content.len()).into_bytes(),
	content].concat()
}

/// Map the root resource to index.html, all other routes treated otherwise.
/// Decode URL encoded spaces (%20) to ensure local file names match.
fn resolve_route<'a>(
	route: &'a str
) -> String {
	match route.replace("%20", " ").as_str() {
		"/" => format!("{}{}", PUBLIC_PFX, "/index.html"),
		route => format!("{}{}", PUBLIC_PFX, route),
	}
}

/// Handle a given bytes buffer `buf` by parsing it. If successful, 
/// determine a local resource from which to construct a response package.
fn handle(buf: &[u8]) -> Vec<u8> {
	match parse(&buf) {
		Ok(req) => {
			dbg!(&req);
			let path = resolve_route(req.route);
			match fs::read(&path) {
				Ok(resource) => package(req.protocol, 200, "OK", resource),
				Err(_) => package_directory(req.protocol, path),
			}
		},
		Err(e) => package("HTTP/1.1", 400, "BAD REQUEST", err_html(400, "BAD REQUEST", e.to_string())),
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
		assert_eq!(req, Request { method: "GET", route: "/", protocol: "HTTP/1.1" });
	}

	#[test]
	fn parse_empty_buffer() {
		let buf = "".as_bytes();
		parse(&buf).unwrap_err();
	}

	#[test]
	fn handle_bad_dir() {
		dir_html(&String::from("hello world")).unwrap_err();
	}
}