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