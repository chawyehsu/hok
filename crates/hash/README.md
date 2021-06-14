scoop-hash
---

TBA

## MD5

**bench**

Compared to [stainless-steel/md5][1], the benchmark script comes from it.

```
> cargo benchcmp .\stainless_steel_md5 .\scoop_hash_md5
 name             stainless_steel_md5 ns/iter  scoop_hash_md5 ns/iter  diff ns/iter   diff %  speedup
 compute_0001000  12,757                       3,931                         -8,826  -69.19%   x 3.25
 compute_0010000  119,832                      35,840                       -83,992  -70.09%   x 3.34
 compute_0100000  1,178,325                    353,450                     -824,875  -70.00%   x 3.33
 compute_1000000  11,790,110                   3,554,690                 -8,235,420  -69.85%   x 3.32
```

*Benchmark ran on Windows 10 with AMD r5 2600, Rust 1.52.1 MSVC*.


[1]: https://github.com/stainless-steel/md5
