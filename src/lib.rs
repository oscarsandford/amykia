use std::{
	io::{Read, BufReader, Write},
	net::TcpStream,
	fs,
};
use crate::cfg::{RECV_BUFFER_SIZE, PUBLIC_PFX, STYLES};

pub mod net;
pub mod cfg;


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

#[derive(Debug)]
struct Request<'a> {
	method: &'a str,
	route: &'a str,
	protocol: &'a str,
}


/// Return simple HTML (in bytes) that serves as a graphical representation of the 
/// directory `dir` with links to navigate to child files and other directories.
fn dir_html(dir_path: &String) -> std::io::Result<Vec<u8>> {
	// This is disgusting, but (should be) safe.
	let relpath = if dir_path.len() >= PUBLIC_PFX.len() { dir_path[PUBLIC_PFX.len()..].trim_end_matches('/') } else {""};
	let backpath = if relpath.len() > 0 {&relpath[..relpath.rfind('/').unwrap_or(relpath.len()-1)]} else {""};
	let mut html = format!("<html><head><title>{relpath}</title><style>{STYLES}</style>\
		</head><body><div class=\"col\"><h1>{relpath}</h1><ul><li><a class=\"dir\" href=\"{backpath}\">../<a></li>").into_bytes();
	for entry in fs::read_dir(dir_path)? {
		let entry = entry?;
		let fname = entry.file_name();
		let fname = fname.to_string_lossy();
		if entry.path().is_dir() { 
			html.extend(&format!("<li><a class=\"dir\" href=\"{relpath}/{fname}\">{fname}/<a></li>").into_bytes());
		}
		else {
			html.extend(&format!("<li><a href=\"{relpath}/{fname}\">{fname}<a></li>").into_bytes());
		}
	}
	html.extend(b"</ul></div></body></html>");
	Ok(html)
}

/// Return simple HTML (in bytes) displaying an error message `e` with a relevant HTTP `code` and `status`.
fn err_html(
	code: u16, 
	status: &str, 
	e: String
) -> Vec<u8> {
	format!("<html><head><title>amykia</title><style>{STYLES}</style></head><body><div class=\"col\"><h1>{code} {status}</h1>{e}</div></body></html>")
	.into_bytes()
}

/// This function is called when there was an error reading the resource as a file.
/// In this case, we try to construct a directory page based on the resource path 
/// `dir` and package it. If still unsuccessful, we package a 404 page.
fn package_directory<'a>(
	protocol: &'a str, 
	dir_path: String
) -> Vec<u8> {
	match dir_html(&dir_path) {
		Ok(html) => package(protocol, 200, "OK", html),
		Err(e) => package(protocol, 404, "NOT FOUND", err_html(404, "NOT FOUND", e.to_string())),
	}
}

/// Return bytes consisting of an HTTP response header joined with its `content` bytes.
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

/// Decode URL encoded spaces (`%20`) to ensure local file names match.
/// Map the root resource (`/`) to `index.html`; all other routes treated as-is.
/// This function ensures that the resulting path string will have a length 
/// greater or equal to the length of the `PUBLIC_PFX`.
fn resolve_route<'a>(
	route: &'a str
) -> String {
	match route.replace("%20", " ").as_str() {
		"/" => format!("{PUBLIC_PFX}/index.html"),
		route => format!("{PUBLIC_PFX}{route}"),
	}
}

/// Parse a HTTP request header from a given bytes buffer `buf`.
fn parse(buf: &[u8]) -> Result<Request, AKErr> {
	let buf_str = std::str::from_utf8(buf)?;
	let lines = buf_str.split("\r\n").collect::<Vec<&str>>();
	let mut first = lines.first().unwrap_or(&"").splitn(3, ' ');
	let method = first.next().ok_or(AKErr::HeaderError("HTTP method unretrievable"))?;
	let route = first.next().ok_or(AKErr::HeaderError("HTTP route unretrievable"))?;
	let protocol = first.next().ok_or(AKErr::HeaderError("HTTP protocol unretrievable"))?;
	let req = Request { method, route, protocol };
	if req.method != "GET" {
		Err(AKErr::HeaderError("Only GET method is currently supported"))
	}
	else { Ok(req) }
}

/// Parse a buffer `buf`. If successful, determine a local resource from 
/// which to `package()` a response in bytes form. If there is an error, 
/// `package()` and return a generic HTML page with error information.
fn respond(buf: &[u8]) -> Vec<u8> {
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

/// Handles a `TcpStream` by reading into a fixed size buffer, 
/// and then calling a function to respond with an indeterminate 
/// number of bytes, and then writing those bytes to the stream.
pub fn handle(mut stream: TcpStream) {
	let mut buf_reader = BufReader::new(&mut stream);
	let mut buf = [0u8; RECV_BUFFER_SIZE];
	if let Err(e) = buf_reader.read(&mut buf) {
		eprintln!("Socket read failed: {:?}", e);
	};
	let res = respond(&buf);
	if let Err(e) = stream.write_all(&res) {
		eprintln!("Socket write failed: {:?}", e);
	}
}


#[cfg(test)]
mod parsing {
    use crate::*;

	#[test]
	fn simple_get() {
		let buf = "GET / HTTP/1.1".as_bytes();
		let req = parse(&buf).unwrap();
		assert_eq!(req.method, "GET");
		assert_eq!(req.route, "/");
		assert_eq!(req.protocol, "HTTP/1.1");
	}

	#[test]
	fn simple_post() {
		let buf = "POST / HTTP/1.1".as_bytes();
		parse(&buf).unwrap_err();
	}

	#[test]
	fn empty_buffer() {
		let buf = "".as_bytes();
		parse(&buf).unwrap_err();
	}
}

#[cfg(test)]
mod resolving {
	use crate::*;

	#[test]
	fn route_resolver() {
		assert_eq!(resolve_route("/"), format!("{PUBLIC_PFX}/index.html"));
		assert_eq!(resolve_route("/asdf"), format!("{PUBLIC_PFX}/asdf"));
		assert_eq!(resolve_route("/jjtk//test"), format!("{PUBLIC_PFX}/jjtk//test"));
	}

	#[test]
	fn invalid_dirs() {
		dir_html(&"".to_string()).unwrap_err();
		dir_html(&" ".to_string()).unwrap_err();
		dir_html(&" ".repeat(PUBLIC_PFX.len())).unwrap_err();
		dir_html(&"...".to_string()).unwrap_err();
	}

	#[test]
	fn valid_dirs() {
		dir_html(&".".to_string()).unwrap();
		dir_html(&"..".to_string()).unwrap();
		dir_html(&PUBLIC_PFX.to_string()).unwrap();
	}
}