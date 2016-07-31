[![Crates.io](https://img.shields.io/crates/v/rz80.svg)](https://crates.io/crates/rz80)
[![Build Status](https://travis-ci.org/floooh/rz80.svg?branch=master)](https://travis-ci.org/floooh/rz80)

# rz80 (work in progress)

Z80 chip family emulator library written in Rust.

[Documentation](https://floooh.github.com/rz80/rz80/index.html)

## Usage
```toml
# Cargo.toml
[dependencies]
rz80 = "0.1.1"
```

## Examples

Run the ZEXDOC and ZEXALL conformance tests:

```bash
> cargo test --release -- --nocapture --ignored
```

Run the [Z1013 home computer emulator](examples/z1013.rs):

```bash
> cargo run --release --example z1013
```

In the Z1013 emulator, start the BASIC interpreter with:

```
# J 300[Enter]
```

The BASIC interpreter will startup and ask for MEMORY SIZE, just hit Enter.

Enter and run a simple Hello World program:

```basic
>AUTO[Enter]
10 FOR I=0 TO 10[Enter]
20 PRINT "HELLO WORLD!"[Enter]
30 NEXT[Enter]
40 [Escape]
OK
>LIST[Enter]
...
>RUN[Enter]
...
>BYE[Enter]
```

