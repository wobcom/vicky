# Vicky

Vicky, which is the babysitter of Timmy, Cosmo and Wanda, is a CD tool for environments with many constraints and dependencies that usually cannot be represented.


## Components

Vicky consists out of multiple components to make a spreaded deployment possible.

+ vicky
    + Main Task Scheduler
+ vicky-worker
    + Task Worker, can run multiple times.
+ dashboard
    + Web-UI
+ vicky-cli
    + CLI

Each component can be developed and deployed individually.

## Concepts

We use an etcd cluster to sync state between multiple instances of Vicky. Vicky will do leader election, so at each time only one instance is active. We try to make Vicky as resilient to network and other failues as possible but it is not our main goal, yet.
All data in the etcd is stored under `vicky.wobcom.de/` in YAML format. 

## Development Setup

We need to start at least a `vicky` instance, S3 storage and etcd to run anything.

### Storage & Database & Certificates

#### docker-compose

+ Generate TLS client certificates for etcd authentication
    + `nix run .\#generate-certs`
    + Certificates are located at `certs`
+ Enter `deployment`
+ Start docker-compose collection
    + `docker-compose up -d` 

#### devenv

TODO @yu-re-ka: Add Information

### Vicky

+ Copy `vicky/Rocket.example.toml` to `vicky/Rocket.toml`
    + `Rocket.example.toml` contains the correct configuration to run with the provided development environment.
+ Edit `vicky/Rocket.toml`
    + Add own machine token to configuration
        + This is needed for `vicky-worker` later.
    + Add OIDC authentication provider to configuration
+ Enter `vicky`
+ Run `cargo run --bin vicky`


### Vicky Worker

+ Copy `vicky-worker/Rocket.example.toml` to `vicky-worker/Rocket.toml`
+ Edit `vicky-worker/Rocket.toml`
    + Add `machine_token` from last step into this configuration.
+ Enter `vicky-worker`
+ Run `cargo run --bin vicky-worker`

### Dashboard

+ Enter `dashboard`
+ Install Dependencies
    + `npm ci` in `dashboard` Folder
+ Run `npm run start`

### CLI

TODO: Add Content for CLI configuration and development.



