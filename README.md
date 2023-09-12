# PI - Process Information

A work-in-progress graphical process and system monitor for Linux

# TODO

- Test on somewhat diverse computers

### UI

- proc: Color cells by amount
- proc: Keyboard shortcuts for scrolling
- proc: Search

### DATA

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

### PERFORMANCE

- plot culling of sys tab

## Note on procfs-reading-performance

- settled for using `pread64`
- mmap seems to be unsupported, gives EIO. (And it wouldn't make sense given the `single_open`/`show` internal linux API)
- io_uring through tokio-uring is just barely on par with the sequential implementation after using sqpoll. No reason to use.
