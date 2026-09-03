# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 496.0 ms | 305.5 ms | `317811` |
| lists.ting | 190.8 ms | 125.3 ms | `100000 0` |
| maps.ting | 211.1 ms | 216.3 ms | `100000 4999950000` |
| stdlib.ting | 835.2 ms | 785.7 ms | `10006 10 500 w0 18974763` |
| strings.ting | 107.1 ms | 95.4 ms | `60000 588890` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
