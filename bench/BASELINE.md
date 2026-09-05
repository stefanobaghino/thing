# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 515.0 ms | 327.7 ms | `317811` |
| json.ting | 142.8 ms | 106.9 ms | `586934 1256961 499950 4 3` |
| lists.ting | 189.5 ms | 123.3 ms | `100000 0` |
| maps.ting | 208.1 ms | 133.4 ms | `100000 4999950000` |
| regex.ting | 235.5 ms | 197.8 ms | `24000 5989512 37 109` |
| stdlib.ting | 866.8 ms | 476.5 ms | `10006 10 500 w0 18974763` |
| strings.ting | 103.4 ms | 72.3 ms | `60000 588890` |
| toplevel.ting | 416.4 ms | 252.6 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
