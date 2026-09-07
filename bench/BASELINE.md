# Benchmark baseline

Median of 5 runs of the release binary (`python3 bench/run.py`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 74.6 ms | 44.9 ms | `80000 30000 22890 11999` |
| fib.ting | 535.1 ms | 336.0 ms | `317811` |
| growth.ting | 239.8 ms | 99.7 ms | `200000 99419 600000` |
| json.ting | 155.1 ms | 115.1 ms | `586934 1256961 499950 4 3` |
| lists.ting | 207.2 ms | 130.6 ms | `100000 0` |
| maps.ting | 214.9 ms | 142.5 ms | `100000 4999950000` |
| regex.ting | 247.6 ms | 207.9 ms | `24000 5989512 37 109` |
| stdlib.ting | 259.6 ms | 161.7 ms | `10006 10 500 w0 18974763` |
| strings.ting | 105.8 ms | 80.0 ms | `60000 588890` |
| toplevel.ting | 468.1 ms | 256.1 ms | `1199980 97 200 10 2062` |

Recorded on: Linux-6.12.34+rpt-rpi-2712-aarch64-with-glibc2.36 / aarch64
