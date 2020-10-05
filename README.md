# verify_server

## Further iterations to `verify_server`:
1. implement `/get_provider_rank` endpoint
1. add time offset to `VerificationRepo.get_provider_rank`
1. add time offset to `VerificationRepo.get_time_since_last_failure(carrier: String)`
1. implement `Best` load balancer
1. implement gateway to route traffic between `RoundRobin` and `Best` verification servers
