# Vicky

Vicky, which is the babysitter of Timmy, Cosmo and Wanda, is a CD tool for environments with many constraints and dependencies that usually cannot be represented.

## Background

We use an etcd cluster to sync state between multiple instances of Vicky. Vicky will do leader election, so at each time only one instance is active. We try to make Vicky as resilient to network and other failues as possible but it is not our main goal, yet.
All data in the etcd is stored under `vicky.wobcom.de/` in YAML format. 

## Development Usage

+ Start etcd in Docker
    + `cd deployment`
    + `docker-compose up -d`
+ Start vicky
    + `cargo run --bin vicky`

Make sure to set the correct rust log flag according to your needs.