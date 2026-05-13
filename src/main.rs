use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use serde_json::json;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb_types::{Array, RecordId, SurrealValue, Value};
use tokio::sync::Barrier;
use tokio::time::sleep;
use uuid::Uuid;
use xxhash_rust::xxh3::xxh3_64;

mod html_report;

type Db = Surreal<Client>;

const QUEUE: &str = "bench";
const MAX_ID_INT: i64 = i64::MAX;

#[derive(Debug, Parser, Clone)]
#[command(version, about = "SurrealDB hash-bucket queue benchmark")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8000")]
    endpoint: String,

    #[arg(long, default_value = "root")]
    user: String,

    #[arg(long, default_value = "root")]
    pass: String,

    #[arg(long, default_value = "queue_bench")]
    namespace: String,

    #[arg(long, default_value = "queue_bench")]
    database: String,

    #[arg(long, default_value_t = 50_000)]
    tasks: u64,

    #[arg(long, value_enum, default_value_t = BenchMode::Concurrent)]
    mode: BenchMode,

    #[arg(long, alias = "num-total-jobs")]
    num_total_jobs: Option<u64>,

    #[arg(long, default_value_t = 4)]
    producers: usize,

    #[arg(long, default_value_t = 4)]
    consumers: usize,

    #[arg(long, alias = "shards", default_value_t = 1024)]
    buckets: u32,

    #[arg(long, alias = "producer-batch-size", default_value_t = 250)]
    producer_batch: usize,

    #[arg(long, alias = "receive-batch-size", default_value_t = 100)]
    consumer_batch: usize,

    #[arg(long, default_value_t = 0)]
    payload_bytes: usize,

    #[arg(long, default_value_t = 30)]
    lease_secs: u64,

    #[arg(long, default_value_t = 0)]
    job_ms: u64,

    #[arg(long, value_name = "NAME")]
    run: Option<String>,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long = "export", alias = "export-html", value_name = "HTML")]
    export: Option<PathBuf>,

    #[arg(long)]
    no_html: bool,

    #[arg(long)]
    separate_clients: bool,

    #[arg(long, default_value_t = true)]
    reset: bool,

    #[arg(long)]
    matrix: bool,

    #[arg(long, value_delimiter = ',', value_name = "N[,N...]")]
    matrix_tasks: Vec<u64>,

    #[arg(long, value_delimiter = ',', value_name = "N[,N...]")]
    matrix_producers: Vec<usize>,

    #[arg(
        long,
        alias = "matrix-workers",
        value_delimiter = ',',
        value_name = "N[,N...]"
    )]
    matrix_consumers: Vec<usize>,

    #[arg(
        long,
        alias = "matrix-shards",
        value_delimiter = ',',
        value_name = "N[,N...]"
    )]
    matrix_buckets: Vec<u32>,

    #[arg(
        long,
        alias = "matrix-producer-batch-size",
        value_delimiter = ',',
        value_name = "N[,N...]"
    )]
    matrix_producer_batches: Vec<usize>,

    #[arg(
        long,
        alias = "matrix-receive-batch-size",
        value_delimiter = ',',
        value_name = "N[,N...]"
    )]
    matrix_receive_batches: Vec<usize>,

    #[arg(long, value_delimiter = ',', value_name = "MS[,MS...]")]
    matrix_job_ms: Vec<u64>,

    #[arg(long, value_delimiter = ',', value_name = "MODE[,MODE...]")]
    matrix_modes: Vec<BenchMode>,

    #[arg(long, default_value_t = 1)]
    matrix_repeats: usize,

    #[arg(long)]
    matrix_explain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum BenchMode {
    InsertOnly,
    DrainOnly,
    Concurrent,
}

#[derive(Debug, Clone)]
struct JobInsert {
    payload: JobPayloadInsert,
    ready: ReadyTokenInsert,
}

#[derive(Debug, Clone, Serialize, SurrealValue)]
struct JobPayloadInsert {
    id: RecordId,
    queue: &'static str,
    job_key: String,
    bucket: u32,
    state: &'static str,
    sent_at_ns: i64,
    payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, SurrealValue)]
struct ReadyTokenInsert {
    id: RecordId,
    queue: &'static str,
    bucket: u32,
    available_ns: i64,
    job: RecordId,
    job_key: String,
    attempts: u32,
    max_attempts: u32,
    sent_at_ns: i64,
}

#[derive(Debug, Deserialize, SurrealValue)]
struct ClaimedJob {
    lease_token: RecordId,
    sent_at_ns: i64,
}

#[derive(Debug, Deserialize, SurrealValue)]
struct WorkerRow {
    worker_id: String,
}

#[derive(Debug, Clone)]
struct BucketState {
    bucket: u32,
    empty_misses: u32,
    ready_at: Instant,
}

#[derive(Debug, Default)]
struct WorkerStats {
    claimed: u64,
    empty_polls: u64,
    bucket_refreshes: u64,
    latency_ns: Vec<u64>,
}

#[derive(Debug)]
struct BenchResult {
    inserted: u64,
    consumed: u64,
    insert_elapsed: Duration,
    total_elapsed: Duration,
    worker_stats: Vec<WorkerStats>,
    counters: Arc<BenchCounters>,
    latency_ns: Vec<u64>,
    mode: BenchMode,
}

#[derive(Debug, Default)]
struct BenchCounters {
    producer_batches_started: AtomicU64,
    producer_batches_completed: AtomicU64,
    receive_calls_started: AtomicU64,
    receive_calls_completed: AtomicU64,
    ack_batches_started: AtomicU64,
    ack_batches_completed: AtomicU64,
    producer_errors: AtomicU64,
    receive_errors: AtomicU64,
    ack_errors: AtomicU64,
    producer_query_ns: AtomicU64,
    producer_query_samples: Mutex<Vec<u64>>,
    select_query_ns: AtomicU64,
    select_query_samples: Mutex<Vec<u64>>,
    ack_query_ns: AtomicU64,
    ack_query_samples: Mutex<Vec<u64>>,
    register_worker_ns: AtomicU64,
    register_worker_count: AtomicU64,
    register_worker_samples: Mutex<Vec<u64>>,
    heartbeat_ns: AtomicU64,
    heartbeat_count: AtomicU64,
    heartbeat_samples: Mutex<Vec<u64>>,
    bucket_refresh_ns: AtomicU64,
    bucket_refresh_count: AtomicU64,
    bucket_refresh_samples: Mutex<Vec<u64>>,
}

#[derive(Debug, Serialize)]
struct JsonReport {
    version: &'static str,
    generated_at_unix_secs: u64,
    config: JsonConfig,
    summary: JsonSummary,
    timings: JsonTimings,
    latency: JsonLatency,
    explain: JsonExplain,
}

#[derive(Debug, Serialize)]
struct MatrixReport {
    version: &'static str,
    generated_at_unix_secs: u64,
    config: MatrixConfig,
    runs: Vec<MatrixRunReport>,
}

#[derive(Debug, Serialize)]
struct MatrixConfig {
    endpoint: String,
    namespace: String,
    database: String,
    queue: &'static str,
    total_runs: usize,
    repeats: usize,
    matrix_tasks: Vec<u64>,
    matrix_producers: Vec<usize>,
    matrix_consumers: Vec<usize>,
    matrix_buckets: Vec<u32>,
    matrix_producer_batches: Vec<usize>,
    matrix_receive_batches: Vec<usize>,
    matrix_job_ms: Vec<u64>,
    matrix_modes: Vec<BenchMode>,
    payload_bytes: usize,
    lease_secs: u64,
    separate_clients: bool,
    queue_model: &'static str,
    captured_explain: bool,
}

#[derive(Debug, Serialize)]
struct MatrixRunReport {
    run_index: usize,
    repeat: usize,
    report: JsonReport,
}

#[derive(Debug, Serialize)]
struct JsonConfig {
    endpoint: String,
    namespace: String,
    database: String,
    queue: &'static str,
    num_total_jobs: u64,
    producers: usize,
    consumers: usize,
    producer_batch_size: usize,
    receive_batch_size: usize,
    shards: u32,
    buckets: u32,
    payload_bytes: usize,
    lease_secs: u64,
    job_ms: u64,
    separate_clients: bool,
    fixed_workdown: bool,
    queue_model: &'static str,
    mode: BenchMode,
}

#[derive(Debug, Serialize)]
struct JsonSummary {
    elapsed_secs: f64,
    producer_window_secs: f64,
    produced_total: u64,
    acked_total: u64,
    final_backlog: i64,
    drained: bool,
    produced_per_sec: f64,
    acked_per_sec: f64,
    producer_batches_started: u64,
    producer_batches_completed: u64,
    producer_batches_in_flight: u64,
    receive_calls_started: u64,
    receive_calls_completed: u64,
    receive_calls_in_flight: u64,
    ack_batches_started: u64,
    ack_batches_completed: u64,
    ack_batches_in_flight: u64,
    producer_errors: u64,
    receive_errors: u64,
    ack_errors: u64,
    empty_polls: u64,
    bucket_refreshes: u64,
}

#[derive(Debug, Serialize)]
struct JsonTimings {
    producer_query: JsonOpTiming,
    select_query: JsonOpTiming,
    ack_query: JsonOpTiming,
    register_worker: JsonOpTiming,
    heartbeat: JsonOpTiming,
    bucket_refresh: JsonOpTiming,
}

#[derive(Debug, Serialize)]
struct JsonOpTiming {
    count: u64,
    total_ms: f64,
    avg_ms: Option<f64>,
    min_ms: Option<f64>,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p99_ms: Option<f64>,
    max_ms: Option<f64>,
    samples_ms: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct JsonLatency {
    count: u64,
    min_ms: Option<f64>,
    mean_ms: Option<f64>,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p99_ms: Option<f64>,
    max_ms: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
struct JsonExplain {
    receive_ready_range: Option<String>,
    recover_lease_range: Option<String>,
    errors: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = Args::parse();
    if let Some(num_total_jobs) = args.num_total_jobs {
        args.tasks = num_total_jobs;
    }
    resolve_run_paths(&mut args)?;
    validate_args(&args)?;

    if args.is_matrix() {
        let report = run_matrix_command(&args).await?;
        write_matrix_report(&args, &report)?;
        html_report::write_html_report(&args, &report)?;
        return Ok(());
    }

    let report = run_single_report(&args).await?;
    write_report(&args, &report)?;
    html_report::write_html_report(&args, &report)?;
    Ok(())
}

impl Args {
    fn is_matrix(&self) -> bool {
        self.matrix
            || !self.matrix_tasks.is_empty()
            || !self.matrix_producers.is_empty()
            || !self.matrix_consumers.is_empty()
            || !self.matrix_buckets.is_empty()
            || !self.matrix_producer_batches.is_empty()
            || !self.matrix_receive_batches.is_empty()
            || !self.matrix_job_ms.is_empty()
            || !self.matrix_modes.is_empty()
            || self.matrix_repeats > 1
    }
}

fn resolve_run_paths(args: &mut Args) -> Result<()> {
    let Some(name) = args.run.as_ref() else {
        return Ok(());
    };
    if name.is_empty() || name.contains('/') || name.contains('\\') {
        anyhow::bail!("--run name must be non-empty and contain no path separators");
    }
    let dir = PathBuf::from("runs").join(name);
    fs::create_dir_all(&dir)
        .with_context(|| format!("create run directory {}", dir.display()))?;
    if args.output.is_none() {
        args.output = Some(dir.join("report.json"));
    }
    if args.export.is_none() && !args.no_html {
        args.export = Some(dir.join("report.html"));
    }
    Ok(())
}

async fn run_single_report(args: &Args) -> Result<JsonReport> {
    let wants_report = args.output.is_some() || html_report::html_output_path(&args).is_some();
    run_report(args, wants_report, true).await
}

async fn run_report(args: &Args, capture_query_plan: bool, print: bool) -> Result<JsonReport> {
    let db = connect(args).await?;
    setup_schema(&db, args).await?;

    let result = run_benchmark(args.clone()).await?;
    if print {
        print_result(args, &result);
    }
    let explain = if capture_query_plan {
        capture_explain(&db, args).await
    } else {
        JsonExplain::default()
    };
    Ok(build_json_report(args, &result, explain))
}

async fn run_matrix_command(base: &Args) -> Result<MatrixReport> {
    let matrix_tasks = matrix_values(&base.matrix_tasks, base.tasks);
    let matrix_producers = matrix_values(&base.matrix_producers, base.producers);
    let matrix_consumers = matrix_values(&base.matrix_consumers, base.consumers);
    let matrix_buckets = matrix_values(&base.matrix_buckets, base.buckets);
    let matrix_producer_batches = matrix_values(&base.matrix_producer_batches, base.producer_batch);
    let matrix_receive_batches = matrix_values(&base.matrix_receive_batches, base.consumer_batch);
    let matrix_job_ms = matrix_values(&base.matrix_job_ms, base.job_ms);
    let matrix_modes = if base.matrix_modes.is_empty() {
        vec![if base.num_total_jobs.is_some() {
            BenchMode::DrainOnly
        } else {
            base.mode
        }]
    } else {
        base.matrix_modes.clone()
    };

    let total_runs = matrix_tasks.len()
        * matrix_producers.len()
        * matrix_consumers.len()
        * matrix_buckets.len()
        * matrix_producer_batches.len()
        * matrix_receive_batches.len()
        * matrix_job_ms.len()
        * matrix_modes.len()
        * base.matrix_repeats;
    println!("matrix runs: {total_runs}");

    let mut runs = Vec::with_capacity(total_runs);
    let mut run_index = 0usize;
    for repeat in 1..=base.matrix_repeats {
        for &tasks in &matrix_tasks {
            for &mode in &matrix_modes {
                for &producers in &matrix_producers {
                    for &consumers in &matrix_consumers {
                        for &buckets in &matrix_buckets {
                            for &producer_batch in &matrix_producer_batches {
                                for &consumer_batch in &matrix_receive_batches {
                                    for &job_ms in &matrix_job_ms {
                                        run_index += 1;
                                        let mut run_args = base.clone();
                                        run_args.tasks = tasks;
                                        run_args.mode = mode;
                                        run_args.num_total_jobs =
                                            (mode == BenchMode::DrainOnly).then_some(tasks);
                                        run_args.producers = producers;
                                        run_args.consumers = consumers;
                                        run_args.buckets = buckets;
                                        run_args.producer_batch = producer_batch;
                                        run_args.consumer_batch = consumer_batch;
                                        run_args.job_ms = job_ms;
                                        run_args.output = None;
                                        run_args.export = None;
                                        run_args.no_html = true;
                                        run_args.reset = true;

                                        println!(
                                            "matrix {run_index}/{total_runs}: mode={mode:?} tasks={tasks} producers={producers} consumers={consumers} buckets={buckets} producer_batch={producer_batch} receive_batch={consumer_batch} job_ms={job_ms} repeat={repeat}"
                                        );
                                        let report =
                                            run_report(&run_args, base.matrix_explain, true)
                                                .await
                                                .with_context(|| {
                                                    format!(
                                                        "matrix run {run_index}/{total_runs} failed"
                                                    )
                                                })?;
                                        runs.push(MatrixRunReport {
                                            run_index,
                                            repeat,
                                            report,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(MatrixReport {
        version: env!("CARGO_PKG_VERSION"),
        generated_at_unix_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        config: MatrixConfig {
            endpoint: base.endpoint.clone(),
            namespace: base.namespace.clone(),
            database: base.database.clone(),
            queue: QUEUE,
            total_runs,
            repeats: base.matrix_repeats,
            matrix_tasks,
            matrix_producers,
            matrix_consumers,
            matrix_buckets,
            matrix_producer_batches,
            matrix_receive_batches,
            matrix_job_ms,
            matrix_modes,
            payload_bytes: base.payload_bytes,
            lease_secs: base.lease_secs,
            separate_clients: true,
            queue_model: "record_range_minimal_tokens",
            captured_explain: base.matrix_explain,
        },
        runs,
    })
}

fn matrix_values<T>(configured: &[T], fallback: T) -> Vec<T>
where
    T: Copy,
{
    if configured.is_empty() {
        vec![fallback]
    } else {
        configured.to_vec()
    }
}

fn validate_args(args: &Args) -> Result<()> {
    if !args.is_matrix() && args.num_total_jobs.is_some() && args.mode == BenchMode::InsertOnly {
        bail!("--num-total-jobs is a drain/workdown input; use --tasks with --mode insert-only");
    }
    if args.producers == 0 {
        bail!("--producers must be greater than zero");
    }
    if args.consumers == 0 {
        bail!("--consumers must be greater than zero");
    }
    if args.buckets == 0 {
        bail!("--buckets must be greater than zero");
    }
    if args.producer_batch == 0 {
        bail!("--producer-batch must be greater than zero");
    }
    if args.consumer_batch == 0 {
        bail!("--consumer-batch must be greater than zero");
    }
    if args.matrix_repeats == 0 {
        bail!("--matrix-repeats must be greater than zero");
    }
    validate_non_zero_u64("matrix-tasks", &args.matrix_tasks)?;
    validate_non_zero_usize("matrix-producers", &args.matrix_producers)?;
    validate_non_zero_usize("matrix-consumers", &args.matrix_consumers)?;
    validate_non_zero_u32("matrix-buckets", &args.matrix_buckets)?;
    validate_non_zero_usize("matrix-producer-batches", &args.matrix_producer_batches)?;
    validate_non_zero_usize("matrix-receive-batches", &args.matrix_receive_batches)?;
    Ok(())
}

fn validate_non_zero_u64(name: &str, values: &[u64]) -> Result<()> {
    if values.contains(&0) {
        bail!("--{name} entries must be greater than zero");
    }
    Ok(())
}

fn validate_non_zero_usize(name: &str, values: &[usize]) -> Result<()> {
    if values.contains(&0) {
        bail!("--{name} entries must be greater than zero");
    }
    Ok(())
}

fn validate_non_zero_u32(name: &str, values: &[u32]) -> Result<()> {
    if values.contains(&0) {
        bail!("--{name} entries must be greater than zero");
    }
    Ok(())
}

async fn connect(args: &Args) -> Result<Db> {
    let db = Surreal::new::<Ws>(&args.endpoint)
        .with_capacity(100_000)
        .await
        .with_context(|| format!("connect to SurrealDB at {}", args.endpoint))?;

    db.signin(Root {
        username: args.user.clone(),
        password: args.pass.clone(),
    })
    .await
    .context("sign in to SurrealDB")?;

    db.use_ns(&args.namespace)
        .use_db(&args.database)
        .await
        .context("select namespace/database")?;

    Ok(db)
}

async fn setup_schema(db: &Db, args: &Args) -> Result<()> {
    let reset = if args.reset {
        r#"
        REMOVE FUNCTION fn::bench_receive_ready;
        REMOVE FUNCTION fn::bench_recover_buckets;
        REMOVE FUNCTION fn::bench_ack;
        REMOVE FUNCTION fn::bench_fail;
        REMOVE FUNCTION fn::bench_receive_bucket;
        REMOVE TABLE job;
        REMOVE TABLE job_payload;
        REMOVE TABLE job_ready;
        REMOVE TABLE job_lease;
        REMOVE TABLE worker;
        "#
    } else {
        ""
    };

    let schema = r#"
        __RESET__

        DEFINE TABLE IF NOT EXISTS worker SCHEMAFULL;
        DEFINE FIELD worker_id ON worker TYPE string;
        DEFINE FIELD queue ON worker TYPE string;
        DEFINE FIELD lease_until ON worker TYPE datetime;
        DEFINE FIELD capacity ON worker TYPE int DEFAULT 1;
        DEFINE FIELD load ON worker TYPE int DEFAULT 0;

        DEFINE TABLE IF NOT EXISTS job_payload SCHEMAFULL;
        DEFINE FIELD queue ON job_payload TYPE string;
        DEFINE FIELD job_key ON job_payload TYPE string;
        DEFINE FIELD bucket ON job_payload TYPE int;
        DEFINE FIELD state ON job_payload TYPE string ASSERT $value INSIDE ["active", "done", "dead"];
        DEFINE FIELD sent_at_ns ON job_payload TYPE int;
        DEFINE FIELD completed_at_ns ON job_payload TYPE option<int>;
        DEFINE FIELD dead_at_ns ON job_payload TYPE option<int>;
        DEFINE FIELD payload ON job_payload TYPE object FLEXIBLE;

        DEFINE TABLE IF NOT EXISTS job_ready SCHEMAFULL;
        DEFINE FIELD queue ON job_ready TYPE string;
        DEFINE FIELD bucket ON job_ready TYPE int;
        DEFINE FIELD available_ns ON job_ready TYPE int;
        DEFINE FIELD job ON job_ready TYPE record<job_payload>;
        DEFINE FIELD job_key ON job_ready TYPE string;
        DEFINE FIELD attempts ON job_ready TYPE int DEFAULT 0;
        DEFINE FIELD max_attempts ON job_ready TYPE int DEFAULT 5;
        DEFINE FIELD sent_at_ns ON job_ready TYPE int;

        DEFINE TABLE IF NOT EXISTS job_lease SCHEMAFULL;
        DEFINE FIELD queue ON job_lease TYPE string;
        DEFINE FIELD bucket ON job_lease TYPE int;
        DEFINE FIELD lease_until_ns ON job_lease TYPE int;
        DEFINE FIELD job ON job_lease TYPE record<job_payload>;
        DEFINE FIELD job_key ON job_lease TYPE string;
        DEFINE FIELD attempts ON job_lease TYPE int DEFAULT 1;
        DEFINE FIELD max_attempts ON job_lease TYPE int DEFAULT 5;
        DEFINE FIELD sent_at_ns ON job_lease TYPE int;

        DEFINE FUNCTION OVERWRITE fn::bench_receive_ready(
            $queue: string,
            $bucket: int,
            $limit: int,
            $available_until_ns: int,
            $lease_until_ns: int
        ) -> array<object> {
            LET $ready = DELETE (
                SELECT VALUE id
                FROM job_ready:[$queue, $bucket, NONE, NONE]..=[$queue, $bucket, $available_until_ns, "~~~~"]
                LIMIT $limit
            ) RETURN BEFORE;
            IF $ready.len() = 0 {
                RETURN [];
            };

            LET $leases = $ready.map(|$row| {
                id: type::record("job_lease", [$queue, $bucket, $lease_until_ns, $row.job_key]),
                queue: $queue,
                bucket: $bucket,
                lease_until_ns: $lease_until_ns,
                job: $row.job,
                job_key: $row.job_key,
                attempts: $row.attempts + 1,
                max_attempts: $row.max_attempts,
                sent_at_ns: $row.sent_at_ns
            });
            INSERT INTO job_lease $leases
            RETURN NONE;

            RETURN $ready.map(|$row| {
                lease_token: type::record("job_lease", [$queue, $bucket, $lease_until_ns, $row.job_key]),
                sent_at_ns: $row.sent_at_ns
            });
        };

        DEFINE FUNCTION OVERWRITE fn::bench_recover_buckets(
            $queue: string,
            $buckets: array<int>,
            $now_ns: int,
            $limit: int
        ) {
            FOR $bucket IN $buckets {
                LET $expired = DELETE (
                    SELECT VALUE id
                    FROM job_lease:[$queue, $bucket, NONE, NONE]..=[$queue, $bucket, $now_ns, "~~~~"]
                    LIMIT $limit
                ) RETURN BEFORE;
                IF $expired.len() > 0 {
                    LET $retry = $expired[WHERE attempts < max_attempts];
                    IF $retry.len() > 0 {
                        LET $ready = $retry.map(|$row| {
                            id: type::record("job_ready", [$queue, $bucket, $now_ns, $row.job_key]),
                            queue: $queue,
                            bucket: $bucket,
                            available_ns: $now_ns,
                            job: $row.job,
                            job_key: $row.job_key,
                            attempts: $row.attempts,
                            max_attempts: $row.max_attempts,
                            sent_at_ns: $row.sent_at_ns
                        });
                        INSERT INTO job_ready $ready
                        RETURN NONE;
                    };

                    LET $dead = $expired[WHERE attempts >= max_attempts];
                    IF $dead.len() > 0 {
                        LET $dead_jobs = $dead.map(|$row| $row.job);
                        UPDATE $dead_jobs
                        SET state = "dead",
                            dead_at_ns = $now_ns
                        RETURN NONE;
                    };
                };
            };
            RETURN NONE;
        };

        DEFINE FUNCTION OVERWRITE fn::bench_ack(
            $lease_tokens: array<record<job_lease>>,
            $completed_at_ns: int
        ) -> int {
            LET $leases = DELETE $lease_tokens
                RETURN BEFORE;
            IF $leases.len() = 0 {
                RETURN 0;
            };

            LET $jobs = $leases.map(|$row| $row.job);
            UPDATE $jobs
            SET state = "done",
                completed_at_ns = $completed_at_ns
            RETURN NONE;

            RETURN $leases.len();
        };

        DEFINE FUNCTION OVERWRITE fn::bench_fail(
            $lease_tokens: array<record<job_lease>>,
            $now_ns: int
        ) {
            LET $leases = DELETE $lease_tokens
                RETURN BEFORE;
            IF $leases.len() = 0 {
                RETURN NONE;
            };

            LET $retry = $leases[WHERE attempts < max_attempts];
            IF $retry.len() > 0 {
                LET $ready = $retry.map(|$row| {
                        id: type::record("job_ready", [$row.queue, $row.bucket, $now_ns, $row.job_key]),
                        queue: $row.queue,
                        bucket: $row.bucket,
                        available_ns: $now_ns,
                        job: $row.job,
                        job_key: $row.job_key,
                        attempts: $row.attempts,
                        max_attempts: $row.max_attempts,
                        sent_at_ns: $row.sent_at_ns
                    });
                INSERT INTO job_ready $ready
                RETURN NONE;
            };

            LET $dead = $leases[WHERE attempts >= max_attempts];
            IF $dead.len() > 0 {
                LET $dead_jobs = $dead.map(|$row| $row.job);
                UPDATE $dead_jobs
                SET state = "dead",
                    dead_at_ns = $now_ns
                RETURN NONE;
            };
            RETURN NONE;
        };

        "#
    .replace("__RESET__", reset);

    db.query(schema).await.context("define queue schema")?;

    Ok(())
}

async fn run_benchmark(args: Args) -> Result<BenchResult> {
    let mode = if args.num_total_jobs.is_some() && args.mode == BenchMode::Concurrent {
        BenchMode::DrainOnly
    } else {
        args.mode
    };

    match mode {
        BenchMode::InsertOnly => run_insert_only(args).await,
        BenchMode::DrainOnly => run_fixed_workdown(args).await,
        BenchMode::Concurrent => run_concurrent_benchmark(args).await,
    }
}

async fn run_concurrent_benchmark(args: Args) -> Result<BenchResult> {
    let start = Instant::now();
    let inserted = Arc::new(AtomicU64::new(0));
    let consumed = Arc::new(AtomicU64::new(0));
    let counters = Arc::new(BenchCounters::default());
    let barrier = Arc::new(Barrier::new(args.producers + args.consumers));

    let mut producer_tasks = Vec::with_capacity(args.producers);
    for producer_id in 0..args.producers {
        producer_tasks.push(tokio::spawn(run_producer(
            args.clone(),
            producer_id,
            inserted.clone(),
            counters.clone(),
            barrier.clone(),
            start,
        )));
    }

    let mut consumer_tasks = Vec::with_capacity(args.consumers);
    for consumer_id in 0..args.consumers {
        consumer_tasks.push(tokio::spawn(run_consumer(
            args.clone(),
            consumer_id,
            consumed.clone(),
            counters.clone(),
            barrier.clone(),
            start,
        )));
    }

    let producer_results = try_join_all(producer_tasks)
        .await
        .context("join producer tasks")?;
    for result in producer_results {
        result?;
    }
    let insert_elapsed = start.elapsed();

    let worker_results = try_join_all(consumer_tasks)
        .await
        .context("join consumer tasks")?;
    let mut worker_stats = Vec::with_capacity(worker_results.len());
    for result in worker_results {
        worker_stats.push(result?);
    }
    let latency_ns = collect_worker_latencies(&worker_stats, args.tasks as usize);

    Ok(BenchResult {
        inserted: inserted.load(Ordering::Relaxed),
        consumed: consumed.load(Ordering::Relaxed),
        insert_elapsed,
        total_elapsed: start.elapsed(),
        worker_stats,
        counters,
        latency_ns,
        mode: BenchMode::Concurrent,
    })
}

async fn run_insert_only(args: Args) -> Result<BenchResult> {
    let started = Instant::now();
    let inserted = Arc::new(AtomicU64::new(0));
    let counters = Arc::new(BenchCounters::default());
    let barrier = Arc::new(Barrier::new(args.producers));

    let mut producer_tasks = Vec::with_capacity(args.producers);
    for producer_id in 0..args.producers {
        producer_tasks.push(tokio::spawn(run_producer(
            args.clone(),
            producer_id,
            inserted.clone(),
            counters.clone(),
            barrier.clone(),
            started,
        )));
    }

    let producer_results = try_join_all(producer_tasks)
        .await
        .context("join insert-only producer tasks")?;
    for result in producer_results {
        result?;
    }
    let elapsed = started.elapsed();

    Ok(BenchResult {
        inserted: inserted.load(Ordering::Relaxed),
        consumed: 0,
        insert_elapsed: elapsed,
        total_elapsed: elapsed,
        worker_stats: Vec::new(),
        counters,
        latency_ns: Vec::new(),
        mode: BenchMode::InsertOnly,
    })
}

async fn run_fixed_workdown(args: Args) -> Result<BenchResult> {
    let payload_started = Instant::now();
    let inserted = Arc::new(AtomicU64::new(0));
    let consumed = Arc::new(AtomicU64::new(0));
    let counters = Arc::new(BenchCounters::default());

    let producer_barrier = Arc::new(Barrier::new(args.producers));
    let mut producer_tasks = Vec::with_capacity(args.producers);
    for producer_id in 0..args.producers {
        producer_tasks.push(tokio::spawn(run_producer(
            args.clone(),
            producer_id,
            inserted.clone(),
            counters.clone(),
            producer_barrier.clone(),
            payload_started,
        )));
    }

    let producer_results = try_join_all(producer_tasks)
        .await
        .context("join producer tasks")?;
    for result in producer_results {
        result?;
    }
    let insert_elapsed = payload_started.elapsed();

    let drain_started = Instant::now();
    let consumer_barrier = Arc::new(Barrier::new(args.consumers));
    let mut consumer_tasks = Vec::with_capacity(args.consumers);
    for consumer_id in 0..args.consumers {
        consumer_tasks.push(tokio::spawn(run_consumer(
            args.clone(),
            consumer_id,
            consumed.clone(),
            counters.clone(),
            consumer_barrier.clone(),
            payload_started,
        )));
    }

    let worker_results = try_join_all(consumer_tasks)
        .await
        .context("join consumer tasks")?;
    let mut worker_stats = Vec::with_capacity(worker_results.len());
    for result in worker_results {
        worker_stats.push(result?);
    }
    let latency_ns = collect_worker_latencies(&worker_stats, args.tasks as usize);

    Ok(BenchResult {
        inserted: inserted.load(Ordering::Relaxed),
        consumed: consumed.load(Ordering::Relaxed),
        insert_elapsed,
        total_elapsed: drain_started.elapsed(),
        worker_stats,
        counters,
        latency_ns,
        mode: BenchMode::DrainOnly,
    })
}

fn collect_worker_latencies(worker_stats: &[WorkerStats], capacity: usize) -> Vec<u64> {
    let mut latency_ns = Vec::with_capacity(capacity);
    for stats in worker_stats {
        latency_ns.extend_from_slice(&stats.latency_ns);
    }
    latency_ns
}

async fn run_producer(
    args: Args,
    producer_id: usize,
    inserted: Arc<AtomicU64>,
    counters: Arc<BenchCounters>,
    barrier: Arc<Barrier>,
    started: Instant,
) -> Result<()> {
    let db = connect(&args).await?;
    barrier.wait().await;

    let mut batch = Vec::with_capacity(args.producer_batch);
    let pad = "x".repeat(args.payload_bytes);
    for task_id in (producer_id as u64..args.tasks).step_by(args.producers) {
        batch.push(make_job(task_id, args.buckets, started, &pad));
        if batch.len() == args.producer_batch {
            insert_jobs(&db, &mut batch, &inserted, &counters).await?;
        }
    }
    insert_jobs(&db, &mut batch, &inserted, &counters).await?;
    Ok(())
}

async fn insert_jobs(
    db: &Db,
    batch: &mut Vec<JobInsert>,
    inserted: &AtomicU64,
    counters: &BenchCounters,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    let count = batch.len() as u64;
    let jobs = std::mem::take(batch);
    let mut payloads = Vec::with_capacity(jobs.len());
    let mut ready = Vec::with_capacity(jobs.len());
    for job in jobs {
        payloads.push(job.payload);
        ready.push(job.ready);
    }
    counters
        .producer_batches_started
        .fetch_add(1, Ordering::Relaxed);
    let query_started = Instant::now();
    let mut response = db
        .query(
            r#"
        BEGIN TRANSACTION;

        INSERT INTO job_payload $payloads
        RETURN NONE;

        INSERT INTO job_ready $ready
        RETURN NONE;

        COMMIT TRANSACTION;
        "#,
        )
        .bind(("payloads", payloads))
        .bind(("ready", ready))
        .await
        .context("insert job batch")?;
    let errors = response.take_errors();
    if !errors.is_empty() {
        counters.producer_errors.fetch_add(1, Ordering::Relaxed);
        bail!("insert job batch: {errors:?}");
    }
    observe_duration(
        &counters.producer_query_ns,
        &counters.producer_query_samples,
        query_started.elapsed(),
    );

    counters
        .producer_batches_completed
        .fetch_add(1, Ordering::Relaxed);
    inserted.fetch_add(count, Ordering::Relaxed);
    Ok(())
}

fn make_job(task_id: u64, buckets: u32, started: Instant, pad: &str) -> JobInsert {
    let hash_key = format!("task:{task_id}");
    let hash = xxh3_64(hash_key.as_bytes());
    let bucket = (hash % buckets as u64) as u32;
    let sent_at_ns = elapsed_ns(started).min(i64::MAX as u64) as i64;
    let job_key = format!("{hash:016x}:{task_id}");
    let payload_id = payload_record_id(&job_key);
    JobInsert {
        payload: JobPayloadInsert {
            id: payload_id.clone(),
            queue: QUEUE,
            job_key: job_key.clone(),
            bucket,
            state: "active",
            sent_at_ns,
            payload: make_payload(pad),
        },
        ready: ReadyTokenInsert {
            id: ready_record_id(bucket, 0, &job_key),
            queue: QUEUE,
            bucket,
            available_ns: 0,
            job: payload_id,
            job_key,
            attempts: 0,
            max_attempts: 5,
            sent_at_ns,
        },
    }
}

fn make_payload(pad: &str) -> serde_json::Value {
    if pad.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        json!({ "pad": pad })
    }
}

fn payload_record_id(job_key: &str) -> RecordId {
    RecordId::new("job_payload", job_key.to_owned())
}

fn ready_record_id(bucket: u32, available_ns: i64, job_key: &str) -> RecordId {
    array_record_id(
        "job_ready",
        vec![
            QUEUE.to_owned().into_value(),
            i64::from(bucket).into_value(),
            available_ns.into_value(),
            job_key.to_owned().into_value(),
        ],
    )
}

fn array_record_id(table: &str, values: Vec<Value>) -> RecordId {
    RecordId::new(table, Array::from(values))
}

fn lease_literal(args: &Args) -> String {
    format!("{}s", args.lease_secs)
}

fn unix_ns_i64() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(i64::MAX as u128) as i64
}

fn elapsed_ns(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
}

fn observe_duration(total_ns: &AtomicU64, samples: &Mutex<Vec<u64>>, elapsed: Duration) {
    let ns = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
    total_ns.fetch_add(ns, Ordering::Relaxed);
    if let Ok(mut samples) = samples.lock() {
        samples.push(ns);
    }
}

async fn run_consumer(
    args: Args,
    consumer_id: usize,
    consumed: Arc<AtomicU64>,
    counters: Arc<BenchCounters>,
    barrier: Arc<Barrier>,
    started: Instant,
) -> Result<WorkerStats> {
    let db = connect(&args).await?;
    let worker_id = format!("worker-{consumer_id}-{}", Uuid::new_v4());
    let mut stats = WorkerStats {
        latency_ns: Vec::with_capacity(
            (args.tasks as usize / args.consumers.max(1)).saturating_add(args.consumer_batch),
        ),
        ..WorkerStats::default()
    };
    let mut bucket_states = Vec::new();
    let heartbeat_interval = Duration::from_secs((args.lease_secs / 3).max(1));
    let mut last_heartbeat = Instant::now();
    let mut last_refresh = Instant::now() - heartbeat_interval;
    let mut bucket_cursor = 0usize;

    register_worker_instrumented(&db, &worker_id, &args, &counters).await?;
    barrier.wait().await;

    while consumed.load(Ordering::Relaxed) < args.tasks {
        if bucket_states.is_empty() || last_refresh.elapsed() >= heartbeat_interval {
            let owned_buckets = refresh_buckets(&db, &worker_id, &args, &counters).await?;
            stats.bucket_refreshes += 1;
            bucket_states = merge_bucket_states(bucket_states, owned_buckets);
            bucket_cursor = bucket_cursor.min(bucket_states.len().saturating_sub(1));
            last_refresh = Instant::now();
        }

        if bucket_states.is_empty() {
            stats.empty_polls += 1;
            sleep(Duration::from_millis(10)).await;
            continue;
        }

        if last_heartbeat.elapsed() >= heartbeat_interval {
            heartbeat(&db, &worker_id, &args, &counters).await?;
            last_heartbeat = Instant::now();
        }

        let Some(index) = next_ready_bucket_index(&bucket_states, bucket_cursor) else {
            stats.empty_polls += 1;
            sleep(next_bucket_sleep(&bucket_states)).await;
            continue;
        };
        bucket_cursor = (index + 1) % bucket_states.len();
        let bucket = bucket_states[index].bucket;
        let jobs = receive_bucket_jobs(&db, bucket, &args, &counters).await?;
        if jobs.is_empty() {
            stats.empty_polls += 1;
            bucket_states[index].empty_misses = bucket_states[index].empty_misses.saturating_add(1);
            bucket_states[index].ready_at =
                Instant::now() + bucket_empty_backoff(bucket_states[index].empty_misses);
            continue;
        }
        bucket_states[index].empty_misses = 0;
        bucket_states[index].ready_at = Instant::now();

        complete_jobs(&db, jobs, &args, &counters, &consumed, &mut stats, started).await?;
    }

    Ok(stats)
}

async fn complete_jobs(
    db: &Db,
    jobs: Vec<ClaimedJob>,
    args: &Args,
    counters: &BenchCounters,
    consumed: &AtomicU64,
    stats: &mut WorkerStats,
    started: Instant,
) -> Result<()> {
    if args.job_ms > 0 {
        sleep(Duration::from_millis(args.job_ms)).await;
    }

    let (claimed, batch_latencies) = ack_jobs(db, jobs, counters, started).await?;
    if claimed == 0 {
        stats.empty_polls += 1;
        sleep(Duration::from_millis(1)).await;
        return Ok(());
    }

    stats.latency_ns.extend(batch_latencies);
    stats.claimed += claimed;
    consumed.fetch_add(claimed, Ordering::Relaxed);
    Ok(())
}

fn merge_bucket_states(previous: Vec<BucketState>, owned_buckets: Vec<u32>) -> Vec<BucketState> {
    let mut previous_by_bucket: HashMap<u32, BucketState> = previous
        .into_iter()
        .map(|state| (state.bucket, state))
        .collect();
    owned_buckets
        .into_iter()
        .map(|bucket| {
            previous_by_bucket.remove(&bucket).unwrap_or(BucketState {
                bucket,
                empty_misses: 0,
                ready_at: Instant::now(),
            })
        })
        .collect()
}

fn next_ready_bucket_index(bucket_states: &[BucketState], cursor: usize) -> Option<usize> {
    let now = Instant::now();
    for offset in 0..bucket_states.len() {
        let index = (cursor + offset) % bucket_states.len();
        if bucket_states[index].ready_at <= now {
            return Some(index);
        }
    }
    None
}

fn next_bucket_sleep(bucket_states: &[BucketState]) -> Duration {
    let now = Instant::now();
    bucket_states
        .iter()
        .filter_map(|state| state.ready_at.checked_duration_since(now))
        .min()
        .unwrap_or_else(|| Duration::from_millis(1))
        .min(Duration::from_millis(10))
}

fn bucket_empty_backoff(empty_misses: u32) -> Duration {
    let capped = empty_misses.min(5);
    Duration::from_millis(1 << capped)
}

async fn register_worker(db: &Db, worker_id: &str, args: &Args) -> Result<()> {
    let lease = lease_literal(args);
    let query = format!(
        r#"
        UPSERT type::record("worker", $worker_id)
        SET
            worker_id = $worker_id,
            queue = $queue,
            lease_until = time::now() + {lease},
            capacity = $capacity
        RETURN NONE;
        "#
    );
    let mut response = db
        .query(query)
        .bind(("worker_id", worker_id.to_owned()))
        .bind(("queue", QUEUE))
        .bind(("capacity", args.consumer_batch as i64))
        .await
        .context("register worker")?;
    let errors = response.take_errors();
    if !errors.is_empty() {
        bail!("register worker: {errors:?}");
    }
    Ok(())
}

async fn register_worker_instrumented(
    db: &Db,
    worker_id: &str,
    args: &Args,
    counters: &BenchCounters,
) -> Result<()> {
    let started = Instant::now();
    register_worker(db, worker_id, args).await?;
    observe_duration(
        &counters.register_worker_ns,
        &counters.register_worker_samples,
        started.elapsed(),
    );
    counters
        .register_worker_count
        .fetch_add(1, Ordering::Relaxed);
    Ok(())
}

async fn heartbeat(db: &Db, worker_id: &str, args: &Args, counters: &BenchCounters) -> Result<()> {
    let lease = lease_literal(args);
    let query = format!(
        r#"
        UPDATE type::record("worker", $worker_id)
        SET lease_until = time::now() + {lease}
        RETURN NONE;
        "#
    );
    let started = Instant::now();
    let mut response = db
        .query(query)
        .bind(("worker_id", worker_id.to_owned()))
        .await
        .context("heartbeat worker")?;
    let errors = response.take_errors();
    if !errors.is_empty() {
        bail!("heartbeat worker: {errors:?}");
    }
    observe_duration(
        &counters.heartbeat_ns,
        &counters.heartbeat_samples,
        started.elapsed(),
    );
    counters.heartbeat_count.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

async fn refresh_buckets(
    db: &Db,
    worker_id: &str,
    args: &Args,
    counters: &BenchCounters,
) -> Result<Vec<u32>> {
    let started = Instant::now();
    let selected_buckets = partition_lease_targets(db, worker_id, args).await?;
    if selected_buckets.is_empty() {
        observe_duration(
            &counters.bucket_refresh_ns,
            &counters.bucket_refresh_samples,
            started.elapsed(),
        );
        counters
            .bucket_refresh_count
            .fetch_add(1, Ordering::Relaxed);
        return Ok(Vec::new());
    }
    recover_expired_leases(db, &selected_buckets, args).await?;
    observe_duration(
        &counters.bucket_refresh_ns,
        &counters.bucket_refresh_samples,
        started.elapsed(),
    );
    counters
        .bucket_refresh_count
        .fetch_add(1, Ordering::Relaxed);
    Ok(selected_buckets)
}

async fn recover_expired_leases(db: &Db, buckets: &[u32], args: &Args) -> Result<()> {
    if buckets.is_empty() {
        return Ok(());
    }
    let bucket_values: Vec<i64> = buckets.iter().map(|bucket| i64::from(*bucket)).collect();
    let mut response = db
        .query(
            r#"
            BEGIN TRANSACTION;
            RETURN fn::bench_recover_buckets($queue, $buckets, $now_ns, $limit);
            COMMIT TRANSACTION;
            "#,
        )
        .bind(("queue", QUEUE))
        .bind(("buckets", bucket_values))
        .bind(("now_ns", unix_ns_i64()))
        .bind(("limit", (args.consumer_batch * 4).max(1) as i64))
        .await
        .context("recover expired leases")?;
    let errors = response.take_errors();
    if !errors.is_empty() {
        bail!("recover expired leases: {errors:?}");
    }
    Ok(())
}

async fn partition_lease_targets(db: &Db, worker_id: &str, args: &Args) -> Result<Vec<u32>> {
    let mut response = db
        .query(
            r#"
            SELECT worker_id
            FROM worker
            WHERE queue = $queue
              AND lease_until > time::now()
            ORDER BY worker_id ASC;
            "#,
        )
        .bind(("queue", QUEUE))
        .await
        .context("load active workers for partition leasing")?;
    let workers: Vec<WorkerRow> = response.take(0).context("decode active workers")?;
    if workers.is_empty() {
        return Ok(Vec::new());
    }

    let mut buckets = Vec::new();
    for bucket in 0..args.buckets {
        let owner = workers
            .iter()
            .max_by_key(|worker| partition_owner_score(bucket, &worker.worker_id))
            .expect("workers is not empty");
        if owner.worker_id == worker_id {
            buckets.push(bucket);
        }
    }
    Ok(buckets)
}

fn partition_owner_score(bucket: u32, worker_id: &str) -> u64 {
    let mut bytes = Vec::with_capacity(worker_id.len() + 16);
    bytes.extend_from_slice(&bucket.to_le_bytes());
    bytes.extend_from_slice(worker_id.as_bytes());
    xxh3_64(&bytes)
}

async fn receive_bucket_jobs(
    db: &Db,
    bucket: u32,
    args: &Args,
    counters: &BenchCounters,
) -> Result<Vec<ClaimedJob>> {
    let lease_until_ns = unix_ns_i64().saturating_add((args.lease_secs as i64) * 1_000_000_000);
    let query = r#"
        BEGIN TRANSACTION;
        RETURN fn::bench_receive_ready($queue, $bucket, $limit, $available_until_ns, $lease_until_ns);
        COMMIT TRANSACTION;
        "#;
    counters
        .receive_calls_started
        .fetch_add(1, Ordering::Relaxed);
    let started = Instant::now();
    let mut response = db
        .query(query)
        .bind(("queue", QUEUE))
        .bind(("bucket", bucket as i64))
        .bind(("limit", args.consumer_batch as i64))
        .bind(("available_until_ns", MAX_ID_INT))
        .bind(("lease_until_ns", lease_until_ns))
        .await
        .context("receive owned jobs")?;
    observe_duration(
        &counters.select_query_ns,
        &counters.select_query_samples,
        started.elapsed(),
    );
    counters
        .receive_calls_completed
        .fetch_add(1, Ordering::Relaxed);

    let errors = response.take_errors();
    if !errors.is_empty() {
        counters.receive_errors.fetch_add(1, Ordering::Relaxed);
        bail!("receive owned jobs: {errors:?}");
    }

    let rows: Vec<ClaimedJob> = response.take(1usize).context("decode received job keys")?;
    Ok(rows)
}

async fn ack_jobs(
    db: &Db,
    jobs: Vec<ClaimedJob>,
    counters: &BenchCounters,
    started: Instant,
) -> Result<(u64, Vec<u64>)> {
    let sent_at_ns: Vec<i64> = jobs.iter().map(|job| job.sent_at_ns).collect();
    let lease_tokens: Vec<RecordId> = jobs.into_iter().map(|job| job.lease_token).collect();
    let query = r#"
        BEGIN TRANSACTION;
        RETURN fn::bench_ack($lease_tokens, $completed_at_ns);
        COMMIT TRANSACTION;
        "#;
    counters.ack_batches_started.fetch_add(1, Ordering::Relaxed);
    let query_started = Instant::now();
    let mut response = db
        .query(query)
        .bind(("lease_tokens", lease_tokens))
        .bind((
            "completed_at_ns",
            elapsed_ns(started).min(i64::MAX as u64) as i64,
        ))
        .await
        .context("ack jobs")?;
    observe_duration(
        &counters.ack_query_ns,
        &counters.ack_query_samples,
        query_started.elapsed(),
    );
    let errors = response.take_errors();
    if !errors.is_empty() {
        bail!("ack jobs: {errors:?}");
    }

    counters
        .ack_batches_completed
        .fetch_add(1, Ordering::Relaxed);
    let now_ns = elapsed_ns(started);
    let acked: Option<i64> = response.take(1usize).context("decode ack count")?;
    let acked = acked.unwrap_or_default();
    let acked = u64::try_from(acked).context("ack count was negative")?;
    let latencies = sent_at_ns
        .iter()
        .take(acked as usize)
        .map(|sent_at_ns| now_ns.saturating_sub((*sent_at_ns).max(0) as u64))
        .collect();
    Ok((acked, latencies))
}

fn print_result(args: &Args, result: &BenchResult) {
    let insert_secs = result.insert_elapsed.as_secs_f64();
    let total_secs = result.total_elapsed.as_secs_f64();
    let insert_rate = result.inserted as f64 / insert_secs.max(0.000_001);
    let total_rate = result.consumed as f64 / total_secs.max(0.000_001);
    let claimed: u64 = result.worker_stats.iter().map(|stats| stats.claimed).sum();
    let empty_polls: u64 = result
        .worker_stats
        .iter()
        .map(|stats| stats.empty_polls)
        .sum();
    let bucket_refreshes: u64 = result
        .worker_stats
        .iter()
        .map(|stats| stats.bucket_refreshes)
        .sum();
    let latency = summarize_latency(&result.latency_ns);

    println!("surreal queue benchmark");
    println!("endpoint:           {}", args.endpoint);
    println!("tasks:              {}", args.tasks);
    println!("producers:          {}", args.producers);
    println!("consumers:          {}", args.consumers);
    println!("mode:               {:?}", result.mode);
    println!("buckets:            {}", args.buckets);
    println!("producer batch:     {}", args.producer_batch);
    println!("consumer batch:     {}", args.consumer_batch);
    println!("job ms:             {}", args.job_ms);
    println!("inserted:           {}", result.inserted);
    println!("consumed:           {}", result.consumed);
    println!("worker claimed:     {}", claimed);
    println!("insert elapsed:     {:.3}s", insert_secs);
    println!("total elapsed:      {:.3}s", total_secs);
    println!("insert throughput:  {:.0} jobs/s", insert_rate);
    println!("end-to-end rate:    {:.0} jobs/s", total_rate);
    if let Some(p95_ms) = latency.p95_ms {
        println!("latency p95:        {:.3} ms", p95_ms);
    }
    if let Some(p99_ms) = latency.p99_ms {
        println!("latency p99:        {:.3} ms", p99_ms);
    }
    println!("empty polls:        {}", empty_polls);
    println!("bucket refreshes:   {}", bucket_refreshes);
    println!(
        "producer query avg: {:.3} ms",
        avg_ms(
            result.counters.producer_query_ns.load(Ordering::Relaxed),
            result
                .counters
                .producer_batches_completed
                .load(Ordering::Relaxed)
        )
        .unwrap_or(0.0)
    );
    println!(
        "select query avg:   {:.3} ms",
        avg_ms(
            result.counters.select_query_ns.load(Ordering::Relaxed),
            result
                .counters
                .receive_calls_completed
                .load(Ordering::Relaxed)
        )
        .unwrap_or(0.0)
    );
    println!(
        "ack query avg:      {:.3} ms",
        avg_ms(
            result.counters.ack_query_ns.load(Ordering::Relaxed),
            result
                .counters
                .ack_batches_completed
                .load(Ordering::Relaxed)
        )
        .unwrap_or(0.0)
    );
}

async fn capture_explain(db: &Db, args: &Args) -> JsonExplain {
    let mut explain = JsonExplain::default();
    explain.receive_ready_range = match explain_text(
        db,
        r#"
        EXPLAIN ANALYZE FORMAT TEXT
        SELECT id, job_key, sent_at_ns
        FROM job_ready:[$queue, 0, NONE, NONE]..=[$queue, 0, 9223372036854775807, "~~~~"]
        LIMIT $limit;
        "#,
        args,
    )
    .await
    {
        Ok(plan) => Some(plan),
        Err(error) => {
            explain
                .errors
                .push(format!("receive_ready_range: {error:#}"));
            None
        }
    };
    explain.recover_lease_range = match explain_text(
        db,
        r#"
        EXPLAIN ANALYZE FORMAT TEXT
        SELECT id, job_key, sent_at_ns
        FROM job_lease:[$queue, 0, NONE, NONE]..=[$queue, 0, $now_ns, "~~~~"]
        LIMIT $limit;
        "#,
        args,
    )
    .await
    {
        Ok(plan) => Some(plan),
        Err(error) => {
            explain
                .errors
                .push(format!("recover_lease_range: {error:#}"));
            None
        }
    };
    explain
}

async fn explain_text(db: &Db, query: &str, args: &Args) -> Result<String> {
    let mut response = db
        .query(query)
        .bind(("queue", QUEUE))
        .bind(("limit", args.consumer_batch as i64))
        .bind(("now_ns", unix_ns_i64()))
        .await
        .context("run EXPLAIN ANALYZE")?;
    let plan: Value = response.take(0usize).context("decode EXPLAIN ANALYZE")?;
    Ok(match plan {
        Value::String(text) => text,
        other => format!("{other:?}"),
    })
}

fn write_report(args: &Args, report: &JsonReport) -> Result<()> {
    let Some(path) = &args.output else {
        return Ok(());
    };
    let bytes = serde_json::to_vec_pretty(&report).context("serialize benchmark report")?;
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn write_matrix_report(args: &Args, report: &MatrixReport) -> Result<()> {
    let Some(path) = &args.output else {
        return Ok(());
    };
    let bytes = serde_json::to_vec_pretty(&report).context("serialize matrix benchmark report")?;
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn build_json_report(args: &Args, result: &BenchResult, explain: JsonExplain) -> JsonReport {
    let insert_secs = result.insert_elapsed.as_secs_f64();
    let total_secs = result.total_elapsed.as_secs_f64();
    let producer_batches_started = result
        .counters
        .producer_batches_started
        .load(Ordering::Relaxed);
    let producer_batches_completed = result
        .counters
        .producer_batches_completed
        .load(Ordering::Relaxed);
    let receive_calls_started = result
        .counters
        .receive_calls_started
        .load(Ordering::Relaxed);
    let receive_calls_completed = result
        .counters
        .receive_calls_completed
        .load(Ordering::Relaxed);
    let ack_batches_started = result.counters.ack_batches_started.load(Ordering::Relaxed);
    let ack_batches_completed = result
        .counters
        .ack_batches_completed
        .load(Ordering::Relaxed);
    let empty_polls: u64 = result
        .worker_stats
        .iter()
        .map(|stats| stats.empty_polls)
        .sum();
    let bucket_refreshes: u64 = result
        .worker_stats
        .iter()
        .map(|stats| stats.bucket_refreshes)
        .sum();

    JsonReport {
        version: env!("CARGO_PKG_VERSION"),
        generated_at_unix_secs: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        config: JsonConfig {
            endpoint: args.endpoint.clone(),
            namespace: args.namespace.clone(),
            database: args.database.clone(),
            queue: QUEUE,
            num_total_jobs: args.tasks,
            producers: args.producers,
            consumers: args.consumers,
            producer_batch_size: args.producer_batch,
            receive_batch_size: args.consumer_batch,
            shards: args.buckets,
            buckets: args.buckets,
            payload_bytes: args.payload_bytes,
            lease_secs: args.lease_secs,
            job_ms: args.job_ms,
            separate_clients: true,
            fixed_workdown: args.num_total_jobs.is_some(),
            queue_model: "record_range_minimal_tokens",
            mode: result.mode,
        },
        summary: JsonSummary {
            elapsed_secs: total_secs,
            producer_window_secs: insert_secs,
            produced_total: result.inserted,
            acked_total: result.consumed,
            final_backlog: result.inserted as i64 - result.consumed as i64,
            drained: result.inserted == result.consumed,
            produced_per_sec: result.inserted as f64 / insert_secs.max(0.000_001),
            acked_per_sec: result.consumed as f64 / total_secs.max(0.000_001),
            producer_batches_started,
            producer_batches_completed,
            producer_batches_in_flight: producer_batches_started
                .saturating_sub(producer_batches_completed),
            receive_calls_started,
            receive_calls_completed,
            receive_calls_in_flight: receive_calls_started.saturating_sub(receive_calls_completed),
            ack_batches_started,
            ack_batches_completed,
            ack_batches_in_flight: ack_batches_started.saturating_sub(ack_batches_completed),
            producer_errors: result.counters.producer_errors.load(Ordering::Relaxed),
            receive_errors: result.counters.receive_errors.load(Ordering::Relaxed),
            ack_errors: result.counters.ack_errors.load(Ordering::Relaxed),
            empty_polls,
            bucket_refreshes,
        },
        timings: build_json_timings(&result.counters),
        latency: summarize_latency(&result.latency_ns),
        explain,
    }
}

fn build_json_timings(counters: &BenchCounters) -> JsonTimings {
    JsonTimings {
        producer_query: op_timing(
            counters.producer_batches_completed.load(Ordering::Relaxed),
            counters.producer_query_ns.load(Ordering::Relaxed),
            &counters.producer_query_samples,
        ),
        select_query: op_timing(
            counters.receive_calls_completed.load(Ordering::Relaxed),
            counters.select_query_ns.load(Ordering::Relaxed),
            &counters.select_query_samples,
        ),
        ack_query: op_timing(
            counters.ack_batches_completed.load(Ordering::Relaxed),
            counters.ack_query_ns.load(Ordering::Relaxed),
            &counters.ack_query_samples,
        ),
        register_worker: op_timing(
            counters.register_worker_count.load(Ordering::Relaxed),
            counters.register_worker_ns.load(Ordering::Relaxed),
            &counters.register_worker_samples,
        ),
        heartbeat: op_timing(
            counters.heartbeat_count.load(Ordering::Relaxed),
            counters.heartbeat_ns.load(Ordering::Relaxed),
            &counters.heartbeat_samples,
        ),
        bucket_refresh: op_timing(
            counters.bucket_refresh_count.load(Ordering::Relaxed),
            counters.bucket_refresh_ns.load(Ordering::Relaxed),
            &counters.bucket_refresh_samples,
        ),
    }
}

fn op_timing(count: u64, total_ns: u64, samples: &Mutex<Vec<u64>>) -> JsonOpTiming {
    let sample_snapshot = samples
        .lock()
        .map(|samples| samples.clone())
        .unwrap_or_default();
    let latency = summarize_latency(&sample_snapshot);
    JsonOpTiming {
        count,
        total_ms: ns_to_ms(total_ns),
        avg_ms: avg_ms(total_ns, count),
        min_ms: latency.min_ms,
        p50_ms: latency.p50_ms,
        p95_ms: latency.p95_ms,
        p99_ms: latency.p99_ms,
        max_ms: latency.max_ms,
        samples_ms: downsample_ms(&sample_snapshot, 1_200),
    }
}

fn downsample_ms(samples_ns: &[u64], max_samples: usize) -> Vec<f64> {
    if samples_ns.is_empty() || max_samples == 0 {
        return Vec::new();
    }
    if samples_ns.len() <= max_samples {
        return samples_ns.iter().copied().map(ns_to_ms).collect();
    }

    let chunk_size = samples_ns.len().div_ceil(max_samples);
    samples_ns
        .chunks(chunk_size)
        .map(|chunk| {
            let sum: u128 = chunk.iter().map(|value| u128::from(*value)).sum();
            ns_to_ms((sum / chunk.len() as u128) as u64)
        })
        .collect()
}

fn avg_ms(total_ns: u64, count: u64) -> Option<f64> {
    if count == 0 {
        None
    } else {
        Some(ns_to_ms(total_ns / count))
    }
}

fn summarize_latency(latencies_ns: &[u64]) -> JsonLatency {
    if latencies_ns.is_empty() {
        return JsonLatency {
            count: 0,
            min_ms: None,
            mean_ms: None,
            p50_ms: None,
            p95_ms: None,
            p99_ms: None,
            max_ms: None,
        };
    }

    let mut sorted = latencies_ns.to_vec();
    sorted.sort_unstable();
    let sum: u128 = sorted.iter().map(|value| u128::from(*value)).sum();
    JsonLatency {
        count: sorted.len() as u64,
        min_ms: sorted.first().copied().map(ns_to_ms),
        mean_ms: Some(ns_to_ms((sum / sorted.len() as u128) as u64)),
        p50_ms: Some(percentile_ms(&sorted, 0.50)),
        p95_ms: Some(percentile_ms(&sorted, 0.95)),
        p99_ms: Some(percentile_ms(&sorted, 0.99)),
        max_ms: sorted.last().copied().map(ns_to_ms),
    }
}

fn percentile_ms(sorted_ns: &[u64], percentile: f64) -> f64 {
    let index = ((sorted_ns.len() - 1) as f64 * percentile).round() as usize;
    ns_to_ms(sorted_ns[index])
}

fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}
