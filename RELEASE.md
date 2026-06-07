# Release Process

ittsy follows [Semantic Versioning](https://semver.org/). The version in
`Cargo.toml` and the Git tag must match exactly.

1. Update `Cargo.toml` to the intended `MAJOR.MINOR.PATCH` version.
2. Update user-facing documentation and commit the release changes.
3. Confirm CI passes on `main`.
4. Create and push an annotated tag:

   ```sh
   git tag -a v0.1.1 -m "ittsy v0.1.1"
   git push origin v0.1.1
   ```

The release workflow verifies the version, runs tests, builds the optimized
Intel macOS binary, packages and ad-hoc signs a standalone `.app`, writes a
SHA-256 checksum, and creates a GitHub release with generated notes.

For local bundle verification:

```sh
cargo build --release
scripts/package-macos.sh
open dist/ittsy.app
```

Run the end-to-end launch and keyboard smoke test on a macOS account that has
granted terminal automation access:

```sh
scripts/verify-macos-app.sh
```

The app is not Developer ID signed or notarized yet, so downloaded builds may
require the standard macOS first-open approval.

Do not move or reuse release tags. Publish fixes as a new patch version.
