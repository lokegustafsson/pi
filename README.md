# PI - Process Information

A work-in-progress graphical process and system monitor for Linux

# TODO

- Test on somewhat diverse computers

## UI

- proc: Color cells by amount
- proc: Keyboard shortcuts for scrolling
- proc: Search

## DATA

- proc: libsystemd for lsid tab
- proc: Network tx/rx
- proc: GPU usage
- proc: PSS instead of RSS memory (avoid overcount of shared memory)
- sys/os: add panel with
    * uptime
    * process count
    * thread count
- sys/disk: usage and capacity. Per-folder breakdown?
- sys/cpu: per-cpu temperature?

## PERFORMANCE

- io_uring, or mmap, for quickly reading procfs? (benchmark)
- plot culling of sys tab
- tracing on drop (sysinfo, procinfo, egui update)
