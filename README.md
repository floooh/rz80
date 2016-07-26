# rz80
Experimental Z80 emulator in Rust.

[![Build Status](https://travis-ci.org/floooh/rz80.svg?branch=master)](https://travis-ci.org/floooh/rz80)

This is just for me getting familiar with Rust, nothing fancy yet.

### Run the sample Z1013 emulator (work in progress!)

```bash
> cargo run --release --example z1013
```

### Do something in BASIC:

> NOTE: currently, an American-English keyboard layout is hardcoded

```
# J 300[Enter]
[Enter]
OK
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

### How to run the ZEX conformance tests

```bash
> cargo test --release -- --nocapture --ignored
...
```
