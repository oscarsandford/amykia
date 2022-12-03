use amykia::{
	handle, 
	net::ThreadPool, cfg::Config
};

fn main() {
	let cfg = Config::new(&mut std::env::args());
	let listener = std::net::TcpListener::bind(&cfg.addr)
			.expect("Should be a valid address/port");
	let pool = ThreadPool::new(cfg.workers);
	println!("Amykia ~\n - Environment: {}\n - Address: {}\n - Workers: {}", 
		if cfg!(debug_assertions) {"debug"} else {"release"}, cfg.addr, cfg.workers);
	for stream in listener.incoming() {
		if let Ok(stream) = stream {
			pool.execute(|| {
				handle(stream);
			});
		}
	}
}
