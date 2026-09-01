# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`).
Not a CI gate; regenerate with `--write` on the machine named
below and compare like with like.

| script | median | checksum |
|--------|-------:|----------|
| fib.ting | 295.0 ms | `317811` |
| lists.ting | 101.1 ms | `100000 0` |
| maps.ting | 111.5 ms | `100000 4999950000` |
| strings.ting | 54.2 ms | `60000 588890` |

Recorded on: macOS-26.5-arm64-arm-64bit-Mach-O / arm64
