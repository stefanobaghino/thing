# Benchmark baseline

Median of 5 runs of the release binary (`ting bench/run.ting`),
for both engines. Not a CI gate; regenerate with `--write` on
the machine named below and compare like with like.

| script | eval | vm | checksum |
|--------|-----:|---:|----------|
| accum.ting | 66.3 ms | 33.7 ms | `80000 30000 22890 11999` |
| fib.ting | 568.4 ms | 301.5 ms | `317811` |
| growth.ting | 243.1 ms | 97.0 ms | `200000 99419 600000` |
| json.ting | 165.6 ms | 95.0 ms | `586934 1256961 499950 4 3` |
| lists.ting | 255.5 ms | 121.6 ms | `100000 0` |
| maps.ting | 224.9 ms | 133.9 ms | `100000 4999950000` |
| regex.ting | 235.2 ms | 192.8 ms | `24000 5989512 37 109` |
| scan.ting | 921.7 ms | 363.6 ms | `400000 120000 40000` |
| stdlib.ting | 269.1 ms | 140.2 ms | `10006 10 500 w0 18974763` |
| strings.ting | 107.6 ms | 73.3 ms | `60000 588890` |
| toplevel.ting | 496.9 ms | 221.9 ms | `1199980 97 200 10 2062` |

Recorded on: Linux 6.12.34+rpt-rpi-2712 aarch64
