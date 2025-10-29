use clap::Parser;
use serde::Serialize;
use serde_json;
use std::net::{TcpStream};
use std::io::{Read, Write};

#[derive(Serialize)]
#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long)]
    command: String,

    #[arg(short, long, default_value = "NA")]
    app: String,
}

fn connect_port(port: u16) -> TcpStream {
        let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        let tcp_nodelay = true;
        stream.set_nodelay(tcp_nodelay).unwrap();
        stream
}

fn send_request(request: String) {
    let mut stream = connect_port(8081);
    let mut buf = [0u8; 1024];
    let _ = stream.write_all(request.as_bytes());
    let _ = stream.flush();
    let bytes_read = stream.read(&mut buf).unwrap();
    println!("bytes read = {}", bytes_read);
    let received = std::str::from_utf8(&buf).expect("valid utf8");
    eprintln!("{}", received);
}

fn main() {
    let args = Args::parse();
    match args.command.as_str() {
        "status" => {
        },
        "start" => {
            if args.app == "NA" {
                println!("Application name required");
                std::process::exit(1);
            }
        },
        "stop" => {
            println!("Not implemented");
            std::process::exit(1);
        },
        "restart" => {
            println!("Not implemented");
            std::process::exit(1);
        },
        _ => {
            println!("unrecognized command");
            std::process::exit(1);
        }
    }

    let request = serde_json::to_string(&args).expect("Failed to convert to JSON");
    println!("Serialized JSON: {}", request);
    send_request(request);
}
