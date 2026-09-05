# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 532.6 ms | 358.0 ms | `317811` |
| json.ting | 151.9 ms | 114.4 ms | `586934 1256961 499950 4 3` |
| lists.ting | 209.6 ms | 138.7 ms | `100000 0` |
| maps.ting | 217.9 ms | 139.5 ms | `100000 4999950000` |
| stdlib.ting | 881.4 ms | 507.0 ms | `10006 10 500 w0 18974763` |
| strings.ting | 109.3 ms | 81.5 ms | `60000 588890` |
| toplevel.ting | 435.6 ms | 261.6 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
