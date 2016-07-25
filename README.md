# rz80
Experimental Z80 emulator in Rust.

[![Build Status](https://travis-ci.org/floooh/rz80.svg?branch=master)](https://travis-ci.org/floooh/rz80)

This is just for me getting familiar with Rust, nothing fancy yet.

### Run the sample Z1013 emulator (work in progress!)

```bash
> cargo run --example z1013
```

### How to run the ZEX conformance tests

```bash
> cargo test --release -- --nocapture --ignored
...
```
