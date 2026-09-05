# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| fib.ting | 528.8 ms | 348.7 ms | `317811` |
| json.ting | 143.3 ms | 106.3 ms | `586934 1256961 499950 4 3` |
| lists.ting | 194.1 ms | 134.7 ms | `100000 0` |
| maps.ting | 214.4 ms | 130.1 ms | `100000 4999950000` |
| regex.ting | 261.6 ms | 234.1 ms | `24000 5989512 37 109` |
| stdlib.ting | 862.5 ms | 482.1 ms | `10006 10 500 w0 18974763` |
| strings.ting | 103.6 ms | 74.2 ms | `60000 588890` |
| toplevel.ting | 416.0 ms | 244.1 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
