# Zero To Production (with axum)

Developed following the book [Zero To Production In Rust](https://www.zero2prod.com/) using [axum](https://crates.io/crates/axum).


A summary of the differences:
  - **§3.3**: the chosen web framework is [axum](https://crates.io/crates/axum);
  - **§3.5**: [hyper](https://crates.io/crates/hyper) is used as HTTP client.
  - **§3.8**: docker compose is used to setting up the test environment:
    ```bash
    $ docker compose -f docker-compose.test.yml -p zero2prod up -d
    $ sqlx database reset -y
    ```

