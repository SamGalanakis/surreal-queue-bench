# surreal-queue-bench

Small Rust benchmark for a SurrealDB-backed queue that uses:

- deterministic task hashes
- record-ID range scans for ready and leased job tokens
- hash buckets as virtual ownership partitions
- worker registration and heartbeats
- rendezvous bucket ownership from active worker registrations
- batched producer inserts
- batched consumer claim/complete loops
- optional per-claim job delay for realistic worker throughput tests
- explicit SurrealDB transactions for multi-record queue state changes
- JSON `EXPLAIN ANALYZE` capture for the hot range reads

Start a local SurrealDB instance:

```bash
surreal start memory --user root --pass root --bind 127.0.0.1:18991
```

Run the benchmark:

```bash
cargo run --release -- \
  --endpoint 127.0.0.1:18991 \
  --tasks 10000 \
  --producers 4 \
  --consumers 4 \
  --buckets 256 \
  --producer-batch 250 \
  --consumer-batch 100
```

Run a fixed work-down with sdbjq-compatible workload names and JSON output.
`--num-total-jobs` inserts all jobs before consumers start, so drain throughput
is comparable to sdbjq's fixed River-style work-down mode:

```bash
cargo run --release -- \
  --endpoint 127.0.0.1:18991 \
  --num-total-jobs 10000 \
  --producers 4 \
  --consumers 4 \
  --shards 256 \
  --producer-batch-size 250 \
  --receive-batch-size 100 \
  --payload-bytes 0 \
  --separate-clients \
  --no-html \
  --output /tmp/record-range-workdown-10k.json
```

Useful knobs:

- `--tasks`: total trivial jobs to enqueue and complete.
- `--mode`: `insert-only`, `drain-only`, or `concurrent`.
- `--num-total-jobs`: fixed work-down mode. Inserts all jobs before consumers start.
- `--producers`: concurrent insert workers.
- `--consumers`: concurrent queue workers.
- `--buckets`: hash bucket count. Keep this comfortably above consumer count.
- `--shards`: alias for `--buckets`, for comparison against shard-based queues.
- `--producer-batch`: jobs inserted per SDK query.
- `--producer-batch-size`: alias for `--producer-batch`.
- `--consumer-batch`: jobs selected, leased, and completed per worker loop.
- `--receive-batch-size`: alias for `--consumer-batch`.
- `--payload-bytes`: bytes of string padding included in each job payload.
- `--job-ms`: milliseconds of simulated work between claim and ack. With
  `--receive-batch-size 1`, this models one in-flight job per worker.
- `--output`: optional JSON report path with comparable config, summary, operation timings, latency fields, and query plans.
- `--export`: optional self-contained HTML report path with graphs for the run.
- `--no-html`: do not auto-write an HTML report next to `--output`.
- `--reset=false`: reuse the current schema/data instead of dropping benchmark tables.
- `--matrix`: run a benchmark matrix instead of a single configuration.
- `--matrix-workers`: comma-separated worker/consumer counts.
- `--matrix-producers`: comma-separated producer counts.
- `--matrix-shards`: comma-separated bucket/shard counts.
- `--matrix-producer-batch-size`: comma-separated producer batch sizes.
- `--matrix-receive-batch-size`: comma-separated receive batch sizes.
- `--matrix-modes`: comma-separated modes such as `drain-only,concurrent`.
- `--matrix-job-ms`: comma-separated simulated job durations.
- `--matrix-repeats`: repeat every matrix cell.
- `--matrix-explain`: capture query plans for every matrix run.

If `--output /tmp/run.json` is provided, the benchmark also writes
`/tmp/run.html` unless `--no-html` is set. Use `--export /tmp/report.html` to
choose the HTML path explicitly.

Run a matrix over workers and batch sizes:

```bash
./target/release/surreal-queue-bench \
  --endpoint 127.0.0.1:18991 \
  --matrix \
  --tasks 10000 \
  --matrix-modes drain-only \
  --matrix-workers 1,2,4,8 \
  --matrix-producers 4 \
  --matrix-shards 64,256 \
  --matrix-producer-batch-size 100,250 \
  --matrix-receive-batch-size 1,10,100 \
  --payload-bytes 0 \
  --output /tmp/queue-matrix.json \
  --export /tmp/queue-matrix.html
```

In matrix mode, `--output` is an aggregate JSON file and `--export` is an
aggregate HTML dashboard. Each matrix cell resets the benchmark tables before
running so insert speed, drain speed, and correctness flags are comparable.

The benchmark uses at-least-once queue semantics. `job_payload` stores the
durable body and terminal state (`active`, `done`, or `dead`); `job_ready` and
`job_lease` are the source of truth for queue placement. Enqueue, claim, ack,
and retry/dead-letter transitions are wrapped in SurrealDB transactions so the
corresponding token changes and terminal-state writes commit or roll back
together. Claim removes ready tokens with `DELETE ... RETURN BEFORE`, then
inserts minimal lease tokens. Worker-to-bucket ownership is computed from active
worker registrations; it is scheduling state, not the claim correctness
boundary. Token movement remains the correctness boundary. The benchmark task
body is intentionally trivial unless `--job-ms` is set.

The comparison aliases only normalize benchmark inputs and outputs. This
benchmark still uses its own record-range token model.

The insert path is intentionally uncoupled from worker ownership. Producers only
compute `bucket = hash(job_id) % buckets`, insert the payload into
`job_payload`, and insert a skinny ready token into `job_ready` whose record ID
is `[queue, bucket, available_ns, job_key]`. Those are two physical writes
because the durable payload record and the moving queue-order token serve
different access paths; the transaction is what makes them one logical enqueue.
Workers register separately and compute bucket ownership from the active worker
set with rendezvous hashing. If a worker dies, its worker lease expires, another
worker takes those buckets on refresh, and expired `job_lease` tokens are
deleted-and-returned for retry or dead-letter handling without rewriting the
original payload body.

Run a realistic one-in-flight-job-per-worker profile:

```bash
./target/release/surreal-queue-bench \
  --endpoint 127.0.0.1:18991 \
  --num-total-jobs 10000 \
  --producers 4 \
  --consumers 4 \
  --shards 256 \
  --producer-batch-size 250 \
  --receive-batch-size 1 \
  --job-ms 100 \
  --payload-bytes 0 \
  --output /tmp/record-range-batch1-job100ms.json
```

Run the record-range queue:

```bash
cargo run --release -- \
  --endpoint 127.0.0.1:18991 \
  --tasks 10000 \
  --producers 4 \
  --consumers 4 \
  --shards 256 \
  --producer-batch-size 250 \
  --receive-batch-size 100 \
  --payload-bytes 0 \
  --output /tmp/record-range-10k.json \
  --export /tmp/record-range-10k.html
```

Profile the phases separately:

```bash
./target/release/surreal-queue-bench \
  --endpoint 127.0.0.1:18991 \
  --mode insert-only \
  --tasks 10000 \
  --producers 4 \
  --consumers 4 \
  --shards 256 \
  --producer-batch-size 250 \
  --receive-batch-size 100 \
  --payload-bytes 0 \
  --output /tmp/record-range-insert-only-10k.json
```

The JSON report includes per-operation timing distributions for producer
queries, select queries, ack queries, worker registration, bucket refresh,
downsampled timing samples for charts, and an `explain` block. The HTML report
plots throughput, phase windows, operation percentiles, sparklines, latency,
counters, configuration, and query plans. The receive and lease recovery plans
should show `RecordIdScan`, not table scans or secondary-index scans.
