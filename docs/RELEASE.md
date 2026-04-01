# What is the release status of `ocelot`?

`ocelot` does not currently ship a release artifact.

The repository has workspace and CI infrastructure in place, but no release workflow should be considered stable until the project defines:

- which crate or binary is released
- versioning policy for internal crates
- packaging targets
- release automation requirements

# What should happen before adding release automation?

Before adding a release process, decide:

- whether the project releases libraries, binaries, or both
- whether all internal crates share one version or version independently
- which targets need first-class support
- what should be included in release artifacts

Until those decisions exist, keep release work manual and repository-local.
