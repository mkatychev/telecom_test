use anyhow::{anyhow, Error};
use rouille::{Request, RequestBody, Response};
use telecom::*;

fn main() {
    let args: Command = argh::from_env();
    let address = format!("localhost:{}", args.port);
    println!("Now listening on {}", address);
    rouille::start_server(address, move |request| {
        let body = telecom::unwrap_request(request);
        match serde_json::from_slice::<VerificationRequest>(&body) {
            Ok(r) => {
                let response = serde_json::to_string(&r).expect("unable to cast to string");
                Response::text(response)
            }
            Err(e) => Response::text(format!(
                "from_slice error - {}:\n\t{}",
                e.to_string(),
                String::from_utf8(body).expect("from_utf8")
            )),
        }
    });
}
