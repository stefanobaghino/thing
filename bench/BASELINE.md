# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 75.0 ms | 43.6 ms | `80000 30000 22890 11999` |
| fib.ting | 535.3 ms | 353.6 ms | `317811` |
| json.ting | 148.1 ms | 105.6 ms | `586934 1256961 499950 4 3` |
| lists.ting | 191.1 ms | 127.7 ms | `100000 0` |
| maps.ting | 214.8 ms | 132.2 ms | `100000 4999950000` |
| regex.ting | 243.5 ms | 194.6 ms | `24000 5989512 37 109` |
| stdlib.ting | 250.9 ms | 153.7 ms | `10006 10 500 w0 18974763` |
| strings.ting | 105.1 ms | 75.3 ms | `60000 588890` |
| toplevel.ting | 451.1 ms | 251.1 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
