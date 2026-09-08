# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 66.9 ms | 34.8 ms | `80000 30000 22890 11999` |
| fib.ting | 545.8 ms | 301.7 ms | `317811` |
| growth.ting | 245.0 ms | 90.9 ms | `200000 99419 600000` |
| json.ting | 154.3 ms | 102.5 ms | `586934 1256961 499950 4 3` |
| lists.ting | 194.1 ms | 119.6 ms | `100000 0` |
| maps.ting | 222.0 ms | 132.4 ms | `100000 4999950000` |
| regex.ting | 234.6 ms | 189.6 ms | `24000 5989512 37 109` |
| scan.ting | 889.6 ms | 355.1 ms | `400000 120000 40000` |
| stdlib.ting | 252.4 ms | 139.7 ms | `10006 10 500 w0 18974763` |
| strings.ting | 108.6 ms | 77.9 ms | `60000 588890` |
| toplevel.ting | 456.5 ms | 229.0 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
