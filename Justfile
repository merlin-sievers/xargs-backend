
run:
    cargo run

build:
    cargo build

build-release:
    cargo build --release

build-container:
    podman build -t xargs-backend .

run-container:
    podman run --rm --name xargs-backend -e RUST_LOG=info -p 127.0.0.1:8484:8484 localhost/xargs-backend:latest /app/target/release/xargs

install-quadlet:
    mkdir -p ~/.config/containers/systemd/
    cp xargs-backend.container ~/.config/containers/systemd
    just build-container
    systemctl --user daemon-reload
    systemctl --user start xargs-backend

