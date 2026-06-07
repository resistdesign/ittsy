# Release Process

tinyterm follows [Semantic Versioning](https://semver.org/). The version in
`Cargo.toml` and the Git tag must match exactly.

1. Update `Cargo.toml` to the intended `MAJOR.MINOR.PATCH` version.
2. Update user-facing documentation and commit the release changes.
3. Confirm CI passes on `main`.
4. Create and push an annotated tag:

   ```sh
   git tag -a v0.1.0 -m "tinyterm v0.1.0"
   git push origin v0.1.0
   ```

The release workflow verifies the version, runs tests, builds the optimized
Intel macOS binary, packages it with the license and README, writes a SHA-256
checksum, and creates a GitHub release with generated notes.

Do not move or reuse release tags. Publish fixes as a new patch version.
