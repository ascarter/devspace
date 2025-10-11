# TODO

- [ ] Container-based sandbox workflow
  - Build Linux target binary (`cargo build --target x86_64-unknown-linux-gnu` etc.)
  - Launch default image (e.g. `quay.io/fedora/fedora-toolbox:latest`) via podman/docker
  - Bind mount repo + compiled `dws` binary into container
  - Start interactive shell with isolated XDG directories; document usage so it can replace the host sandbox

