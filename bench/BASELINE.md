# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 701.1 ms | 449.4 ms | `317811` |
| json.ting | 232.2 ms | 173.8 ms | `586934 1256961 499950 4 3` |
| lists.ting | 256.7 ms | 207.4 ms | `100000 0` |
| maps.ting | 270.8 ms | 246.9 ms | `100000 4999950000` |
| stdlib.ting | 1030.5 ms | 970.8 ms | `10006 10 500 w0 18974763` |
| strings.ting | 132.7 ms | 93.3 ms | `60000 588890` |
| toplevel.ting | 469.4 ms | 309.4 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
