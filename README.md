# verify_server

```
Usage: target/debug/telecom --balancer <balancer> [-p <port>]

Top-level command.

Options:
  --balancer        strategy in selecting what telecom provider handles a
                    verification attempt
  -p, --port        the port that the telecom verification service runs on [
                    default: 5000 ]
  --help            display usage information

```

## Further iterations to `verify_server`:
1. implement `/get_provider_rank` endpoint
1. add time offset to `VerificationRepo.get_provider_rank`
1. add time offset to `VerificationRepo.get_time_since_last_failure(carrier: String)`
1. implement `Best` load balancer
1. implement gateway to route traffic between `RoundRobin` and `Best` verification servers
