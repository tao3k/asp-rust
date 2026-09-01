# Expected workspace Build DAG

Selecting `app` produces the dependency-first package order `shared`, `left`,
`right`, `app`. The shared diamond dependency occurs exactly once. An unknown
selected package, an unadmitted local path dependency, or a local cycle fails
before any package policy gate executes.
