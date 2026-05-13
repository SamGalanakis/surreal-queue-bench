# surreal-queue-bench

A Rust benchmark for a SurrealDB-backed queue built on composite record-IDs.

It uses:

- composite record-IDs as ordering tokens, scanned with `RecordIdScan`
- hash buckets as virtual ownership partitions, mapped to workers via rendezvous hashing
- short-TTL lease tokens claimed with `DELETE … RETURN BEFORE`
- explicit SurrealDB transactions around every multi-record state change
- at-least-once delivery semantics, with payload state and queue tokens kept in separate tables
- a matrix sweep mode and a self-contained HTML report

See [`docs/architecture.html`](docs/architecture.html) for the design walkthrough.

## Run a benchmark

Start SurrealDB locally:

```bash
surreal start memory --user root --pass root --bind 127.0.0.1:18991
```

Run one benchmark and save the report under `runs/`:

```bash
cargo run --release -- \
  --endpoint 127.0.0.1:18991 \
  --run quick \
  --num-total-jobs 10000 \
  --producers 4 \
  --consumers 4 \
  --shards 256
```

This writes:

```
runs/
  quick/
    report.json   # raw timing data, query plans, config
    report.html   # self-contained interactive viewer
```

Open `runs/quick/report.html` in a browser.

## Matrix sweeps

Sweep across regimes, worker counts, and batch sizes in one command:

```bash
cargo run --release -- \
  --endpoint 127.0.0.1:18991 \
  --run holistic \
  --matrix \
  --num-total-jobs 10000 \
  --matrix-modes insert-only,drain-only,concurrent \
  --matrix-workers 1,2,4,8,16,24 \
  --matrix-producers 1,2,4,8,16,24 \
  --matrix-shards 256 \
  --matrix-producer-batch-size 250 \
  --matrix-receive-batch-size 1,10,50,100
```

The HTML report includes a scatter pivot with X/Y/color axis pickers, a
filterable legend, a sortable run table, multi-run overlay, and per-regime
filtering. See [`runs/matrix-holistic/report.html`](runs/matrix-holistic/report.html)
for an example.

## Key flags

| Flag | Purpose |
| --- | --- |
| `--run NAME` | Write `runs/NAME/report.{json,html}`. |
| `--mode` | `insert-only`, `drain-only`, or `concurrent`. |
| `--num-total-jobs N` | Fixed work-down: insert N jobs, then drain. |
| `--producers`, `--consumers` | Concurrent worker counts. |
| `--shards` | Hash bucket count (alias `--buckets`). |
| `--producer-batch-size`, `--receive-batch-size` | Per-loop batch sizes (aliases `--producer-batch`, `--consumer-batch`). |
| `--payload-bytes` | Padding bytes per job payload. |
| `--job-ms` | Simulated work between claim and ack. |
| `--matrix` and `--matrix-*` | Multi-cell sweep over any dimension. |
| `--matrix-repeats N` | Repeat each cell N times. |
| `--matrix-explain` | Capture query plans for every cell. |
| `--no-html` | Skip the HTML report. |
| `--output PATH`, `--export PATH` | Override the auto-derived paths from `--run`. |

## How the queue works

- `job_payload` stores the durable body and terminal state (`active`, `done`,
  `dead`). `job_ready` and `job_lease` carry skinny ordering tokens whose
  record IDs (`[queue, bucket, available_ns, job_key]`) are the source of
  truth for queue placement.
- Enqueue, claim, ack, and retry/dead-letter are all wrapped in SurrealDB
  transactions so token movements and terminal-state writes commit together.
- Claims remove ready tokens with `DELETE … RETURN BEFORE` and insert lease
  tokens; expired leases are reclaimed without rewriting payload bodies.
- Worker-to-bucket ownership is recomputed from active worker registrations
  via rendezvous hashing. It's scheduling state, not the correctness boundary.
- Producers compute `bucket = hash(job_id) % buckets`, write a payload row,
  and insert one ready token — two physical writes, one logical enqueue.

The receive and lease-recovery query plans in `report.json` should show
`RecordIdScan`, not table or secondary-index scans.
