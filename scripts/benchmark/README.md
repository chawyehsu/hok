# Benchmark

This directory contains scripts for benchmarking the performance between Scoop,
Hok, and other implementations. Various commands that are frequently used by
users are benchmarked, including `search`, `list` and `bucket list`.

## Usage

To run the benchmark, you will need to have the **Windows Sandbox** feature enabled.
Then, run the following command to generate a runnable sandbox configuration from
the template:

```powershell
$ ./generate.ps1 # or ./generate.ps1 -Proxy <proxy> to set up a proxy for the benchmark environment
```

After that, a `sandbox.wsb` file will be generated. Double click it to launch
the sandbox, and wait for the benchmark to finish. The benchmark results will
be printed to the console.

The benchmark script `bench.ps1` is generated with the `bench.ps1.template`
template. You can view the template to see how the benchmark is performed.
