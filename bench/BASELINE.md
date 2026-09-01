# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 264.4 ms | 266.4 ms | `317811` |
| lists.ting | 91.7 ms | 91.8 ms | `100000 0` |
| maps.ting | 100.9 ms | 103.1 ms | `100000 4999950000` |
| strings.ting | 50.3 ms | 50.6 ms | `60000 588890` |

Recorded on: macOS-26.5-arm64-arm-64bit-Mach-O / arm64
