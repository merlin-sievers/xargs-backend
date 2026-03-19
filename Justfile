
run:
    cargo run

build:
    cargo build

build-release:
    cargo build --release

build-container:
    podman build -t xargs-backend .
