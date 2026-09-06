# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 536.7 ms | 337.7 ms | `317811` |
| json.ting | 146.9 ms | 119.8 ms | `586934 1256961 499950 4 3` |
| lists.ting | 201.6 ms | 131.3 ms | `100000 0` |
| maps.ting | 207.7 ms | 136.1 ms | `100000 4999950000` |
| regex.ting | 231.9 ms | 197.0 ms | `24000 5989512 37 109` |
| stdlib.ting | 345.6 ms | 196.0 ms | `10006 10 500 w0 18974763` |
| strings.ting | 106.2 ms | 76.4 ms | `60000 588890` |
| toplevel.ting | 424.9 ms | 247.2 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
