use amykia::{
	receive,
	net::ThreadPool
};

const ADDRESS: &str = "127.0.0.1";
const PORT: &str = "5000";
const NUM_WORKERS: usize = 4;

fn main() {
	let listener = std::net::TcpListener::bind(format!("{ADDRESS}:{PORT}"))
			.expect("Should be a valid address/port");
	let pool = ThreadPool::new(NUM_WORKERS);
	println!("~~> Amykia listening on {ADDRESS}:{PORT}");
	for stream in listener.incoming() {
		if let Ok(stream) = stream {
			pool.execute(|| {
				receive(stream);
			});
		}
	}
}
