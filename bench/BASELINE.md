# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 601.0 ms | 335.4 ms | `317811` |
| json.ting | 155.5 ms | 168.3 ms | `586934 1256961 499950 4 3` |
| lists.ting | 206.3 ms | 136.1 ms | `100000 0` |
| maps.ting | 255.1 ms | 221.7 ms | `100000 4999950000` |
| stdlib.ting | 871.3 ms | 820.3 ms | `10006 10 500 w0 18974763` |
| strings.ting | 117.7 ms | 98.8 ms | `60000 588890` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
