pub const RECV_BUFFER_SIZE: usize = 1024;
// Edit this prefix manually or via environment variable (think Docker).
pub const PUBLIC_PFX: &str = "./public"; 

pub const STYLES: &str = "
  :root {
	--primary: #8686f3;
	--secondary: #61a051;
	--light: #e5e5ee;
	--dark: #423433;
	--chalk: #f0f0f0;
  }
  
  body {
	background-color: var(--dark);
	color: var(--light);
	font-family: \"Lucida Console\", \"Courier New\", monospace;
	text-align: center;
	display: flex;
	justify-content: center;
	align-items: center;
  }
  
  .col {
	flex-direction: column;
  }
  
  ul {
	list-style-type: none;
	display: inline-block;
	text-align: left;
  }
  
  li {
	padding: 4px;
  }
  
  a {
	text-decoration: none;
	color: var(--primary);
  }
  
  .dir {
	color: var(--secondary);
  }
  
  .bg {
	width: 70%;
  }";

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