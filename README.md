# Vicky

Vicky consists of three services:
- Vicky itself
- Dashboard (The frontend)
- Vicky runner

## Dev Environment

### Requirements

For a dev environment we need the following services:
- S3 endpoint
- OIDC endpoint
- etcd

There is a Dockerfile and a nix based devenv supplied to run those.

For vicky itself we ship a nix flake that provides all the dependencies.

### Dashboard

```
cd dashboard
npm start
```

Now the frontend listens at `http://localhost:1234/tasks`

### Vicky

Example Rocket.toml for **dev environments**:
```toml
[default]

machines = [
    "abc1234"
]

[default.etcd_config]
endpoints = [ "http://localhost:2379" ]

[default.s3_config]
endpoint = "http://localhost:9000"
access_key_id = "minio"
secret_access_key = "aichudiKohr6aithi4ahh3aeng2eL7xo"
region = "us-east-1"
log_bucket = "vicky-logs"

[default.oauth.github]
provider = "GitHub"
client_id = "CHANGEME"
client_secret = "CHANGEME"
redirect_uri = "http://localhost:1234/api/auth/callback/github"


[default.users.sdinkleberg]
full_name = "Sheldon Dinkleberg"
role = "admin"
```

Now you can start the service running:
```
cargo run --bin vicky 
```

## Create a task

Tasks always have a display name and a `flake_ref`.
The `flake_ref` is a reference to a Nix flake that runs the code.

Example request:
```
      curl -s --request POST \
        --url http://127.0.0.1:1234/api/tasks/ \
        --header "Authorization: abc1234" \
        --header 'Content-Type: application/json' \
        --data '{
          "display_name": "ExampleTask Turn on coffee machine",
          "locks": [],
          "flake_ref": {
              "flake": "github:wobcom/cosmo#generate-certs",
              "args": []
          }
      }'
```


