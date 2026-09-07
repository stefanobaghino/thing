# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 70.5 ms | 38.3 ms | `80000 30000 22890 11999` |
| fib.ting | 561.8 ms | 355.4 ms | `317811` |
| growth.ting | 247.7 ms | 106.9 ms | `200000 99419 600000` |
| json.ting | 190.8 ms | 119.3 ms | `586934 1256961 499950 4 3` |
| lists.ting | 204.6 ms | 125.8 ms | `100000 0` |
| maps.ting | 241.4 ms | 156.2 ms | `100000 4999950000` |
| regex.ting | 242.6 ms | 191.3 ms | `24000 5989512 37 109` |
| scan.ting | 900.7 ms | 384.9 ms | `400000 120000 40000` |
| stdlib.ting | 273.9 ms | 147.2 ms | `10006 10 500 w0 18974763` |
| strings.ting | 125.4 ms | 93.0 ms | `60000 588890` |
| toplevel.ting | 493.9 ms | 238.7 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
