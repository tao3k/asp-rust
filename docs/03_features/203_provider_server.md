# Runtime-managed Rust Provider Server

`asp-rust` has no public provider-local CLI. The ASP Runtime launches the
catalog-declared HTTP/JSON server with the sole admitted process argument:

```text
asp-rust serve
```

Runtime supplies the provider, artifact, registration, contract, language,
and listener identities. The server exposes `GET /health`,
`POST /v1/provider-runtime`, and `POST /shutdown`; business operations are
limited to the operations declared by
`provider/asp-provider-registration.json`: `syntax-query`,
`projection-batch`, and `project-resolution`.

Search, exact query, checks, agent lifecycle, cache lifecycle, generation
admission, and publication are ASP Server ClientFrame operations. The language
package contains no alternate command parser, lifecycle owner, publication
owner, cache, or retry path.

Build the Runtime-managed artifact with:

```shell
cargo build --features provider-server --bin asp-rust
```

The transport admits 4–16 concurrent connections, bounded by available CPU
parallelism. A tracked task owns each admitted connection. Synchronous parser
work runs on an awaited blocking worker; an incomplete request does not block
other connections. Shutdown stops admission and drains admitted connections,
including the shutdown response. Drain waits for those clients to finish.

Run the feature and transport checks with:

```shell
cargo test --workspace --all-targets --all-features --locked
cargo test --workspace --no-default-features --locked
cargo test --no-default-features --features provider-server --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Run explicit release performance profiles with:

```shell
cargo test --release --all-features --lib latency_profile --locked -- --ignored --nocapture --test-threads=1
```

Each profile warms up five times and records 20 samples with nearest-rank
p50/p95/p99. The transport profile compares four sequential requests with four
concurrent requests using the same 10 ms asynchronous handler. It measures
transport scheduling overlap; it does not establish parser or whole-task
speedup. The projection profile parses a fixed checked-in Rust fixture across
16 owners, checks complete results, and reports source digest and response size.
CI saves both JSON receipts in the `provider-latency` artifact. Timing profiles
remain explicit measurements; deterministic barriers enforce concurrency in
the ordinary regression suite.

### Consolidated dependency qualification (2026-10-15)

Local validation on macOS completed with 519 full-feature tests, 499 minimal
feature tests, and 517 provider-only tests passing; explicit latency profiles,
Clippy with denied warnings, and formatting also passed. Disabled dependency
baseline support is tested as an explicit rejection, while enabled support
retains its successful lockfile check.

Five sequential baseline/candidate pairs used the same release test driver,
16 owners of the same 291-byte checked-in fixture, and 48,549 response bytes.
The baseline is `main@9b95fd6744a89f3f38e87f9bda33ea762f1f65e1`.
The fixture BLAKE3 digest is
`769c5d83acbace2b36967050f622717b19c670423572683283efab4bd95c112b`.
Each table value is the median of five per-run percentiles, in milliseconds:

| Version | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Baseline | 0.891 | 1.484 | 1.839 |
| Consolidated dependencies | 0.904 | 1.049 | 1.136 |

Baseline per-run p95 ranged from 0.885 to 2.313 ms; candidate p95 ranged from
0.910 to 1.555 ms. These samples establish a small-case comparison only;
they do not establish a general performance improvement. Earlier measurements
under concurrent compilation had substantially higher tail latency and are
excluded from this paired comparison.

The separate loopback control recorded sequential/concurrent four-request
p95 of 53.072/14.290 ms with the same deliberate 10 ms handler delay and a
12-connection limit. This demonstrates scheduling overlap only.
