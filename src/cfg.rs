pub struct Config {
	pub addr: String,
	pub workers: usize,
}

impl Config {
	pub fn new(args: &mut std::env::Args) -> Config {
		let addr = args.nth(1).unwrap_or("127.0.0.1:5000".to_string());
		let workers = args.next().unwrap_or("4".to_string()).parse::<usize>().unwrap_or(4);
		Config { addr, workers }
	}
}