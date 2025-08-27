`mire` is a program to slow down another program running on Linux without changing the target program's source code.

When launched with the correct set of options, `mire` waits for the target program to begin.

Once the target process (identified by name) is found, `mire` attaches to each of its threads using the Linux ptrace
system call. From this point, for a pre-configured time, the traced program is interrupted for a fixed duration and then
allowed to run for another fixed duration. The result is that the target program slows down. The following command line
options are supported:

```shell
Usage: mire [OPTIONS] --process-name <PROCESS_NAME>

Options:
      --throttle-duration-ms <THROTTLE_DURATION_MS>            [default: 900]
      --free-run-duration-ms <FREE_RUN_DURATION_MS>            [default: 300]
      --total-control-duration-ms <TOTAL_CONTROL_DURATION_MS>  [default: 120000]
      --wait-for-process
      --process-name <PROCESS_NAME>
      --cmd-line-pattern <CMD_LINE_PATTERN>
  -h, --help                                                   Print help
  -V, --version                                                Print version
```

The process name is the name of the target program which is subjected to an exact match. The throttle duration is
milliseconds for which the target is paused _per cycle_. The free run duration is the duration
for which the program runs freely per cycle. Cycles are repeated for total control duration.

A command line pattern can be passed for fine-tuning the process match.
