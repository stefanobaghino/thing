# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 65.7 ms | 33.4 ms | `80000 30000 22890 11999` |
| fib.ting | 544.9 ms | 321.7 ms | `317811` |
| growth.ting | 246.9 ms | 94.7 ms | `200000 99419 600000` |
| json.ting | 140.6 ms | 92.4 ms | `586934 1256961 499950 4 3` |
| lists.ting | 195.0 ms | 120.6 ms | `100000 0` |
| maps.ting | 216.6 ms | 129.2 ms | `100000 4999950000` |
| regex.ting | 235.9 ms | 187.2 ms | `24000 5989512 37 109` |
| scan.ting | 901.0 ms | 364.0 ms | `400000 120000 40000` |
| stdlib.ting | 254.5 ms | 139.8 ms | `10006 10 500 w0 18974763` |
| strings.ting | 103.0 ms | 69.7 ms | `60000 588890` |
| toplevel.ting | 461.7 ms | 227.1 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
