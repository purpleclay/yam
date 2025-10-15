# YAM

`yam` is a context-aware YAML to markdown document generator that parses YAML files
and renders them as markdown tables.

## Install

To install the latest version using a bash script:

```sh
sh -c "$(curl https://raw.githubusercontent.com/purpleclay/yam/main/scripts/install)"
```

Download a specific version using the `-v` flag. The script uses `sudo` by default but can be disabled through the `--no-sudo` flag. You can also provide a different installation directory from the default `/usr/local/bin` by using the `-d` flag:

```sh
sh -c "$(curl https://raw.githubusercontent.com/purpleclay/yam/main/scripts/install)" \
  -- -v 0.1.0 --no-sudo -d ./bin
```

## Run with Nix

If you have nix installed, you can run the binary directly from the GitHub repository:

```sh
nix run github:purpleclay/yam -- --help
```

## Benchmarks

To run the benchmarks:

```sh
cargo bench
```

Benchmarks where run on an Apple M4 Pro (12 cores) with 24GB of RAM:

| Fixture      | Size   | Lines | Mean Time | Throughput (MB/s) | Throughput (lines/s) |
| ------------ | ------ | ----- | --------- | ----------------- | -------------------- |
| external-dns | 50 KB  | 1,206 | ~1.24 ms  | ~40.40 MB/s       | ~974,340             |
| minio        | 76 KB  | 1,749 | ~1.71 ms  | ~44.34 MB/s       | ~1,020,415           |
| redis        | 102 KB | 2,347 | ~2.46 ms  | ~41.44 MB/s       | ~953,570             |

To view the benchmark report:

```sh
open target/criterion/report/index.html
```
