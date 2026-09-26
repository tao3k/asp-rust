# Expected workspace Build DAG

The admitted workspace produces the dependency-first package order `shared`,
`left`, `right`, `app`. The shared diamond dependency occurs exactly once.
External local path dependencies remain external leaves; duplicate member
names and local member cycles fail before any package policy gate executes.
