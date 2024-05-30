# Vicky

Vicky, which is the babysitter of Timmy in *The Fairly OddParents*, is a CD tool for environments with many constraints and dependencies that usually cannot be represented.


## Components

Vicky consists out of multiple components to make a spreaded deployment possible.

+ vicky
    + Main Task Scheduler | *Commands fairies to work on tasks*
+ fairy
    + Fairy, can run multiple times. | *Asks for tasks from vicky and runs them locally*
+ dashboard
    + Web-UI
+ vickyctl
    + CLI application to manage vicky

Each component can be developed and deployed individually. 

## Development Setup

We need to start an instance of `vicky`, S3 storage (here, `minio`) and `postgres` to run anything.

These are provided to you in the `deployment` folder as a docker compose file.

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
        + This is needed for `fairy` later.
    + Add OIDC authentication provider to configuration
+ Enter `vicky`
+ Run `cargo run --bin vicky`


### Fairy

+ Copy `fairy/Rocket.example.toml` to `fairy/Rocket.toml`
+ Edit `fairy/Rocket.toml`
    + Add `machine_token` from last step into this configuration.
+ Enter `fairy`
+ Run `cargo run --bin fairy`


### Dashboard

+ Enter `dashboard`
+ Install Dependencies
    + `npm ci` in `dashboard` Folder
+ Run `npm run start`


### CLI

+ Enter vickyctl
+ Run `cargo run` for help
+ Provide `VICKY_URL` and `VICKY_TOKEN` as env variables to the program so that it can connect to vicky.
    + Example: `VICKY_URL=http://127.0.0.1:8000 VICKY_TOKEN=abc1234 cargo run task create --name "Deployment 1" --flake-url github:wobcom/example-vicky --lock-name "Cool Lock" --lock-type WRITE`
  
```
Usage: vickyctl <COMMAND>

Commands:
  task   Manage tasks on the vicky delegation server
  tasks  Show all tasks vicky is managing
  locks  Show all poisoned locks vicky is managing
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

