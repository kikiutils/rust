use std::{
    future::pending,
    hint::black_box,
};

use criterion::{
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use kikiutils::task::manager::TaskManager;
use tokio::{
    runtime::{
        Builder,
        Runtime,
    },
    spawn,
};

// Constants/Statics
const TASK_COUNTS: &[usize] = &[1, 8, 64, 256];

// Functions
fn build_runtime() -> Runtime {
    Builder::new_current_thread()
        .build()
        .unwrap_or_else(|error| panic!("failed to build Tokio runtime: {error}"))
}

fn throughput(count: usize) -> Throughput {
    Throughput::Elements(u64::try_from(count).unwrap_or(u64::MAX))
}

fn spawn_ready_tasks(manager: &TaskManager, count: usize) {
    for _ in 0..count {
        manager.spawn(async {
            black_box(());
        });
    }
}

fn spawn_yielding_tasks(manager: &TaskManager, count: usize) {
    for _ in 0..count {
        manager.spawn(async {
            tokio::task::yield_now().await;
            black_box(());
        });
    }
}

fn spawn_cancellable_tasks(manager: &TaskManager, count: usize) {
    for _ in 0..count {
        manager.spawn_with_token(|token| async move {
            token.cancelled().await;
            black_box(());
        });
    }
}

fn spawn_pending_tasks(manager: &TaskManager, count: usize) {
    for _ in 0..count {
        manager.spawn(pending::<()>());
    }
}

fn bench_single_task(c: &mut Criterion) {
    let runtime = build_runtime();
    let manager = TaskManager::new();

    let mut group = c.benchmark_group("single_task");

    group.bench_function("tokio_spawn_join", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(spawn(async { black_box(()) }).await.is_ok());
        });
    });

    group.bench_function("manager_spawn_join", |b| {
        b.to_async(&runtime).iter(|| async {
            let task = manager.spawn(async { black_box(()) });
            black_box(task.join().await.is_ok());
        });
    });

    group.bench_function("manager_spawn_with_token_managed_cancel_join", |b| {
        b.to_async(&runtime).iter(|| async {
            let task = manager.spawn_with_token(|token| async move {
                token.cancelled().await;
                black_box(());
            });

            black_box(task.cancel());
            black_box(task.join().await.is_ok());
        });
    });

    group.bench_function("manager_spawn_with_token_manager_cancel_join", |b| {
        b.to_async(&runtime).iter(|| async {
            let task = manager.spawn_with_token(|token| async move {
                token.cancelled().await;
                black_box(());
            });

            black_box(manager.cancel(task.id()));
            black_box(task.join().await.is_ok());
        });
    });

    group.finish();
}

fn bench_spawn_and_join_existing(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("batch_spawn_and_join_existing");

    for &count in TASK_COUNTS {
        group.throughput(throughput(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let manager = TaskManager::new();

            b.to_async(&runtime).iter(|| async {
                spawn_ready_tasks(&manager, count);
                manager.join_existing().await;
                black_box(manager.is_empty());
            });
        });
    }

    group.finish();
}

fn bench_join_existing(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("batch_join_existing");

    for &count in TASK_COUNTS {
        group.throughput(throughput(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let manager = TaskManager::new();

            b.to_async(&runtime).iter(|| async {
                spawn_yielding_tasks(&manager, count);
                manager.join_existing().await;
                black_box(manager.is_empty());
            });
        });
    }

    group.finish();
}

fn bench_cancel_and_join_existing(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("batch_cancel_and_join_existing");

    for &count in TASK_COUNTS {
        group.throughput(throughput(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let manager = TaskManager::new();

            b.to_async(&runtime).iter(|| async {
                spawn_cancellable_tasks(&manager, count);
                manager.cancel_and_join_existing().await;
                black_box(manager.is_empty());
            });
        });
    }

    group.finish();
}

fn bench_abort_and_join_existing(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("batch_abort_and_join_existing");

    for &count in TASK_COUNTS {
        group.throughput(throughput(count));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let manager = TaskManager::new();

            b.to_async(&runtime).iter(|| async {
                spawn_pending_tasks(&manager, count);
                manager.abort_and_join_existing().await;
                black_box(manager.is_empty());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_task,
    bench_spawn_and_join_existing,
    bench_join_existing,
    bench_cancel_and_join_existing,
    bench_abort_and_join_existing,
);
criterion_main!(benches);
