# `telecom` SMS/text-to-speech verification server

```
Usage: telecom --balancer <balancer> [-p <port>]

Top-level command.

Options:
  --balancer        strategy in selecting what telecom provider handles a
                    verification attempt
  -p, --port        the port that the telecom verification service runs on
  --help            display usage information
```

## Running server

Run server with round robin balancer on `localhost:5000`:
`telecom --balancer round-robin -p 5000`

Many mock carrier profiles can be created in `fn main()` with various rates of failure:

```rust
fn main() -> Result<(), Error> {
    // ...
    carriers.push(Box::new(MockTelecomProvider::new("carrier_1", 60, 50)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_2", 50, 60)?));
    carriers.push(Box::new(MockTelecomProvider::new("carrier_3", 10, 100)?));
    // ...
}
```

When defining the behaviour of a `MockTelecomProvider::new("carrier_1", sms, voice)`:
* `sms` arg is the chance that an SMS verificaiton attempt will fail
* `voice` arg is the chance that a text-to-speech verificaiton attempt will fail




## Interacting with server
* Seeding the server with 200 verificaiton attempts: `for i in $(seq 1 200); do curl -d '{"number": "555", "time": '"$(date +%s)"'}' localhost:5000; echo ""; done`
* Returning carrier performance rankings, less is better: `curl -s -X GET localhost:5000/rank`
* Returning the most performant carrier: `curl -s -X GET localhost:5000/rank | jq '.rank[0][0]'`



## Further iterations to `verify_server`:
1. implement `/rank:<time_range>` endpoint to display rankings for past `n` seconds
1. add time offset to `VerificationRepo.get_provider_rank`
1. add time offset to `VerificationRepo.get_time_since_last_failure(carrier: String)`
1. implement `Best` balancer
1. implement gateway to route traffic between `RoundRobin` and `Best` verification servers
