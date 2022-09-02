# angsd-io

[![GitHub Actions status](https://github.com/malthesr/angsd-io/workflows/CI/badge.svg)](https://github.com/malthesr/angsd-io/actions)

**angsd-io** is a collection of Rust crates for reading and writing binary file formats associated with [ANGSD](https://github.com/angsd/angsd).

## Usage

To use one of the **angsd-io** in your own project, add the following to the `[dependencies]` section of your `Cargo.toml`:

```
angsd-saf = { git = "https://github.com/malthesr/angsd-io.git" }
```

Replace `angsd-saf` with the name of the crate you wish to depend on.

For more information, see [here](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories). In particular, you may wish to depend on a specific commit to avoid breakage:

```
angsd-saf = { git = "https://github.com/malthesr/angsd-io.git", rev = "abc" }
```

Replace `abc` with the hash of the commit you wish to depend on.

## Documentation

The documentation can be built and viewed locally by running:

```
cargo doc --open
```
