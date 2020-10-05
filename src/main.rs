use crate::provider::{MockTelecomProvider, TelecomProvider};
use crate::repo::VerificationKeeper;
use crate::VerificationServer;
use anyhow::{anyhow, Error};
use rouille::{router, Request, RequestBody, Response};
use telecom::*;

fn main() -> Result<(), Error> {
    let args: Command = argh::from_env();
    let address = format!("localhost:{}", args.port);
    let mut carriers: Vec<Box<dyn TelecomProvider>> = Vec::new();

    carriers.push(Box::new(MockTelecomProvider::new("carrier_1", 90, 50)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_2", 80, 60)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_3", 95, 20)?));

    let mut keeper =
        Box::new(VerificationKeeper::new([1, 2, 3, 4, 5]).expect("failed to create new keeper"));

    let server = VerificationServer::new(args.balancer, carriers, keeper);
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
