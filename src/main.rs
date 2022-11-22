use amykia::{receive, ThreadPool};
use std::net::TcpListener;

const ADDRESS: &str = "127.0.0.1";
const PORT: &str = "5000";
const WORKERS: usize = 4;

fn main() {
	if let Ok(listener) = TcpListener::bind(format!("{ADDRESS}:{PORT}")) {
		let pool = ThreadPool::new(WORKERS);
		println!("Amykia listening on {ADDRESS}:{PORT}");
		for stream in listener.incoming() {
			if let Ok(stream) = stream {
				pool.execute(|| {
					receive(stream);
				});
			}
		}
	}
}
