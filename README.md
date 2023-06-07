# PI - Process Information

A work-in-progress graphical process and system monitor for Linux

# TODO

- Test on somewhat diverse computers

## UI

- Process list/tree: Color cells by amount
- Process list/tree: Granularity user/processgroup/process/thread
- Process list/tree: Fields CPU/GPU/MEM usage. NET/DISK read/write.
- Process list/tree: Filters

## Missing system Info

- SYSTEM: Handles current / cumulative
- SYSTEM: Threads current / cumulative
- SYSTEM: Processes current / cumulative
- SYSTEM: Uptime
- SYSTEM: Per-cpu temperature (where is this available? not hwmon..)
- MEM: Swap usage and size
- DISK: Usage by folder breakdown? (not real-time obviously)

## Missing Process info (all of it!)

- Granularity user/processgroup/process/thread
- user
- pgrp
- pid
- tid
- name (thread)
- command (process)
- CPU usage (thread)
- number of threads (process)
- GPU usage (process)
- MEM (resident) (process)
- NETWORK rx (process)
- NETWORK tx (process)
- DISK read (process)
- DISK write (process)
