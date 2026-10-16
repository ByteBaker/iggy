<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/apache/iggy/refs/heads/master/assets/logo/SVG/iggy-apache-color-darkbg.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/apache/iggy/refs/heads/master/assets/logo/SVG/iggy-apache-color-lightbg.svg">
    <img alt="Apache Iggy" src="https://raw.githubusercontent.com/apache/iggy/refs/heads/master/assets/logo/SVG/iggy-apache-color-lightbg.svg" width="320">
  </picture>
</div>

# Apache Iggy CLI

[![Crate](https://img.shields.io/crates/v/iggy-cli?logo=rust&style=flat-square)](https://crates.io/crates/iggy-cli)

Interactive CLI for [Apache Iggy](https://iggy.apache.org). Manage streams, topics, partitions, users, consumer groups, and produce/consume messages from your terminal.

## Installation

```bash
cargo install iggy-cli
```

Run `iggy --help` for the full command tree.

## TCP TLS With a Private CA

Pass your CA certificate in PEM format. The TLS domain must match the server certificate:

```bash
iggy --tcp-server-address iggy.example.com:8090 \
  --tcp-tls-enabled --tcp-tls-domain iggy.example.com \
  --tcp-tls-ca-file /path/to/ca.pem ping
```

To save these settings in a context:

```bash
iggy context create production --tcp-server-address iggy.example.com:8090 \
  --tcp-tls-enabled true --tcp-tls-domain iggy.example.com \
  --tcp-tls-ca-file /path/to/ca.pem
iggy context use production
```

Command-line options override the active context. Certificate verification remains enabled.

![CLI](https://raw.githubusercontent.com/apache/iggy/refs/heads/master/assets/cli.png)

## Links

- [Project website](https://iggy.apache.org)
- [GitHub repository](https://github.com/apache/iggy)
- [Iggy SDK on crates.io](https://crates.io/crates/iggy)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](https://github.com/apache/iggy/blob/master/LICENSE).
