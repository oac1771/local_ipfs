
# Development
This project levereges [mise](https://mise.jdx.dev/) to manage dev tools. Follow the directions [here](https://mise.jdx.dev/getting-started.html#quickstart) to install.

Install project dependencies
```
mise install
```

Install Cargo Make
```
cargo install --no-default-features cargo-make
```

# Start Docker Daemon
```
cargo make start-colima
```

# Build

Docker build builder
```
cargo make docker-build-builder 
```

Docker build 
```
cargo make docker-build <component>
```

# Run

Start docker compose

```
cargo make docker-up
```

```
curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "api_ping"}' http://localhost:8080/
```

using ipfs cli inside compose network:
```
ipfs pin ls --api=/dns4/ipfs/tcp/5001
```
