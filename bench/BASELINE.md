# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 74.6 ms | 44.1 ms | `80000 30000 22890 11999` |
| fib.ting | 530.9 ms | 356.1 ms | `317811` |
| json.ting | 150.5 ms | 105.9 ms | `586934 1256961 499950 4 3` |
| lists.ting | 197.7 ms | 132.2 ms | `100000 0` |
| maps.ting | 204.1 ms | 135.7 ms | `100000 4999950000` |
| regex.ting | 241.7 ms | 208.0 ms | `24000 5989512 37 109` |
| stdlib.ting | 340.4 ms | 191.2 ms | `10006 10 500 w0 18974763` |
| strings.ting | 105.2 ms | 75.9 ms | `60000 588890` |
| toplevel.ting | 453.8 ms | 249.0 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
