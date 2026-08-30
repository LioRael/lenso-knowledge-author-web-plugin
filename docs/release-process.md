# Release process

GitHub repository publication and crates.io publication are separate decisions.

1. Run every command in the README verification block from a clean checkout.
2. Confirm every Git dependency uses an immutable full revision and the repository boundary gate finds no path dependency or machine-local path.
3. Merge only after CI is green and verify the remote commit and tree.
4. Allocate the crate name once manually before enabling GitHub Trusted Publishing.
5. Do not add a long-lived registry token. Crates.io publication remains deferred until all required Lenso packages are available from the registry.
