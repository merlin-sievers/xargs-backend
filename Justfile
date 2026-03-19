
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

deploy:
    just build-container
    podman save -o xargs-backend-container.tar localhost/xargs-backend:latest
    rsync xargs-backend-container.tar mrln:/home/xargs-backend/xargs-backend

install-quadlet:
    mkdir -p ~/.config/containers/systemd/
    cp xargs-backend.container ~/.config/containers/systemd
    podman load -i xargs-backend-container.tar
    systemctl --user daemon-reload
    systemctl --user start xargs-backend

