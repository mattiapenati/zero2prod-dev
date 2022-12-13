# Zero To Production (with axum)

Developed following the book [Zero To Production In Rust](https://www.zero2prod.com/) using [axum](https://crates.io/crates/axum).


A summary of the differences:
  - **§3.3**: the chosen web framework is [axum](https://crates.io/crates/axum);
  - **§3.5**: [hyper](https://crates.io/crates/hyper) is used as HTTP client;
  - **§3.8**: docker compose is used to setting up the test environment:
    ```bash
    $ docker compose -f docker-compose.test.yml -p zero2prod up -d
    $ sqlx database reset -y
    ```
  - **§3.9**: `axum::State` is used as type safe replacement of `axum::Extension` and `?` operator is used to handle errors;
  - **§4.5**: traces are collected using [Grafana Tempo](https://grafana.com/oss/tempo/) and they can be inspected using [Grafana](https://grafana.com/) at the address `http://localhost:3000`;
  - **§5.3**: hierarchical configuration is not implemented, configuration can be customized using environment variables; database migrations can be executed on service startup.
