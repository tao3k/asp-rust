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
