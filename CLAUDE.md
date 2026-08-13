# Instructions to Claude

- `cargo add <pkg>` adds crates from menhera-cooldown crates.io proxy, which makes it impossible to publish this crate to crates.io. Please use `cargo add --registry crates-io <pkg>` or edit `Cargo.toml` manually.
- Node.JS/Python usages are discouraged. `python3` invocations will fail.
- avoid non-POSIX shell commands. `POSIXLY_CORRECT` is on for GNU utils. prefer POSIX `/bin/sh`.
- `cURL` is disabled on this machine. `wget` technically works, but its usages are discouraged.
