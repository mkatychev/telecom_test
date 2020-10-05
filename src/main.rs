use crate::provider::{MockTelecomProvider, TelecomProvider};
use crate::repo::VerificationKeeper;
use crate::VerificationServer;
use anyhow::{anyhow, Error};
use rouille::{router, Response};
use std::sync::Mutex;
use telecom::*;

fn main() -> Result<(), Error> {
    let args: Command = argh::from_env();
    let address = format!("localhost:{}", args.port);
    let mut carriers: Vec<Box<dyn TelecomProvider>> = Vec::new();

    carriers.push(Box::new(MockTelecomProvider::new("carrier_1", 60, 50)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_2", 50, 60)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_3", 10, 100)?));

    let keeper =
        Box::new(VerificationKeeper::new([1, 2, 3, 4, 5]).expect("failed to create new keeper"));

    let server = Mutex::new(VerificationServer::new(args.balancer, carriers, keeper));
    println!("Now listening on {}", address);
    rouille::start_server(address, move |request| {
        router!(request,
            // -------------------------
            // POST VERIFICATION ATTEMPT
            // -------------------------
            (POST) (/) => {
                println!("POST /");
                let body = telecom::unwrap_request(request);
                let request = match serde_json::from_slice::<VerificationRequest>(&body) {
                    Ok(r) => r,
                    Err(e) => {
                        return Response::text(format!(
                            "from_slice error - {}:\n\t{}",
                            e.to_string(),
                            String::from_utf8(body).expect("from_utf8")
                        ))
                    }
                };

                match server.lock().unwrap().handle_request(&request) {
                    Ok(r) => return Response::text(r.to_string()),
                    Err(e) => return Response::text(format!("{}", anyhow!(e))),
                }
            },
            // -------------------------
            // GET CARRIER RANKINGS
            // -------------------------
            (GET) (/rank) => {
                println!("GET /rank");
                Response::json(&server.lock().unwrap().get_provider_rank())
            },
            _ => {
                println!("invalid endpoint: {}", request.raw_url());
                Response::text("404")
            }
        )
    });
}
