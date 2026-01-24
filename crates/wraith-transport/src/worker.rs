//! Worker thread pool with CPU pinning and NUMA awareness.
//!
//! Provides a thread pool implementation optimized for network packet processing:
//! - Thread-per-core model with CPU affinity
//! - NUMA-aware memory allocation
//! - Lock-free work distribution
//! - Per-worker statistics tracking
//! - Graceful shutdown handling
//!
//! Target: >95% CPU utilization, scales to 16+ cores

use crate::buffer_pool::BufferPool;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of worker threads (0 = auto-detect from CPU count)
    pub num_workers: usize,
    /// Queue capacity per worker
    pub queue_capacity: usize,
    /// Enable CPU pinning (Linux only)
    pub pin_to_cpu: bool,
    /// Enable NUMA-aware allocation (Linux only)
    pub numa_aware: bool,
    /// Optional buffer pool for packet buffer recycling
    ///
    /// When provided, packet buffers are returned to the pool after processing
    /// instead of being dropped, reducing allocation overhead significantly.
    pub buffer_pool: Option<BufferPool>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            num_workers: 0, // Auto-detect
            queue_capacity: 10000,
            pin_to_cpu: cfg!(target_os = "linux"),
            numa_aware: cfg!(target_os = "linux"),
            buffer_pool: None,
        }
    }
}

impl WorkerConfig {
    /// Create a new WorkerConfig with a buffer pool
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - Size of each buffer in bytes (typically MTU size)
    /// * `pool_size` - Number of buffers to pre-allocate
    ///
    /// # Example
    ///
    /// ```
    /// use wraith_transport::worker::WorkerConfig;
    ///
    /// // Create config with buffer pool for 1500-byte MTU packets
    /// let config = WorkerConfig::with_buffer_pool(1500, 1024);
    /// assert!(config.buffer_pool.is_some());
    /// ```
    #[must_use]
    pub fn with_buffer_pool(buffer_size: usize, pool_size: usize) -> Self {
        Self {
            buffer_pool: Some(BufferPool::new(buffer_size, pool_size)),
            ..Default::default()
        }
    }
}

/// Work item to be processed by a worker
#[derive(Debug)]
pub enum Task {
    /// Process an incoming packet
    ProcessPacket {
        /// Packet data
        data: Vec<u8>,
        /// Source address identifier
        source: usize,
    },
    /// Send an outgoing packet
    SendPacket {
        /// Packet data
        data: Vec<u8>,
        /// Destination address identifier
        destination: usize,
    },
    /// Shutdown the worker
    Shutdown,
}

/// Worker pool for packet processing
///
/// Manages a pool of worker threads pinned to CPU cores for optimal
/// performance in multi-core systems.
pub struct WorkerPool {
    workers: Vec<Worker>,
    task_tx: Sender<Task>,
    shutdown: Arc<AtomicBool>,
    stats: Arc<PoolStats>,
    /// Optional buffer pool for packet buffer recycling
    buffer_pool: Option<BufferPool>,
}

/// Worker thread statistics
#[derive(Debug, Default)]
pub struct WorkerStats {
    /// Total tasks processed
    pub tasks_processed: AtomicU64,
    /// Total packets processed
    pub packets_processed: AtomicU64,
    /// Total bytes processed
    pub bytes_processed: AtomicU64,
    /// Total errors encountered
    pub errors: AtomicU64,
}

/// Pool-wide statistics
#[derive(Debug, Default)]
pub struct PoolStats {
    /// Per-worker statistics
    workers: Vec<Arc<WorkerStats>>,
    /// Pool start time
    start_time: Option<Instant>,
}

impl PoolStats {
    /// Get total tasks processed across all workers
    pub fn total_tasks(&self) -> u64 {
        self.workers
            .iter()
            .map(|w| w.tasks_processed.load(Ordering::Relaxed))
            .sum()
    }

    /// Get total packets processed across all workers
    pub fn total_packets(&self) -> u64 {
        self.workers
            .iter()
            .map(|w| w.packets_processed.load(Ordering::Relaxed))
            .sum()
    }

    /// Get total bytes processed across all workers
    pub fn total_bytes(&self) -> u64 {
        self.workers
            .iter()
            .map(|w| w.bytes_processed.load(Ordering::Relaxed))
            .sum()
    }

    /// Get total errors across all workers
    pub fn total_errors(&self) -> u64 {
        self.workers
            .iter()
            .map(|w| w.errors.load(Ordering::Relaxed))
            .sum()
    }

    /// Get throughput in packets per second
    pub fn packets_per_second(&self) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                self.total_packets() as f64 / elapsed
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Get throughput in bytes per second
    pub fn bytes_per_second(&self) -> f64 {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                self.total_bytes() as f64 / elapsed
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl WorkerPool {
    /// Create a new worker pool with the specified configuration
    ///
    /// # Examples
    /// ```no_run
    /// use wraith_transport::worker::{WorkerPool, WorkerConfig};
    ///
    /// let config = WorkerConfig::default();
    /// let pool = WorkerPool::new(config);
    /// ```
    pub fn new(config: WorkerConfig) -> Self {
        let num_workers = if config.num_workers == 0 {
            num_cpus::get()
        } else {
            config.num_workers
        };

        info!(
            "Creating worker pool with {} workers (queue capacity: {}, buffer_pool: {})",
            num_workers,
            config.queue_capacity,
            if config.buffer_pool.is_some() {
                "enabled"
            } else {
                "disabled"
            }
        );

        let (task_tx, task_rx) = bounded(config.queue_capacity * num_workers);
        let shutdown = Arc::new(AtomicBool::new(false));

        let mut workers = Vec::with_capacity(num_workers);
        let mut worker_stats = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let stats = Arc::new(WorkerStats::default());
            worker_stats.push(stats.clone());

            let worker = Worker::spawn(
                id,
                task_rx.clone(),
                shutdown.clone(),
                stats,
                config.pin_to_cpu,
                config.numa_aware,
                config.buffer_pool.clone(),
            );
            workers.push(worker);
        }

        let pool_stats = Arc::new(PoolStats {
            workers: worker_stats,
            start_time: Some(Instant::now()),
        });

        Self {
            workers,
            task_tx,
            shutdown,
            stats: pool_stats,
            buffer_pool: config.buffer_pool,
        }
    }

    /// Acquire a buffer from the pool
    ///
    /// If a buffer pool is configured, acquires a buffer from the pool.
    /// Otherwise, allocates a new buffer with the specified size.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the buffer to acquire (used when no pool is configured)
    ///
    /// # Example
    ///
    /// ```
    /// use wraith_transport::worker::{WorkerPool, WorkerConfig};
    ///
    /// let config = WorkerConfig::with_buffer_pool(1500, 128);
    /// let pool = WorkerPool::new(config);
    ///
    /// let buffer = pool.acquire_buffer(1500);
    /// assert_eq!(buffer.len(), 1500);
    /// ```
    pub fn acquire_buffer(&self, size: usize) -> Vec<u8> {
        match &self.buffer_pool {
            Some(pool) => pool.acquire(),
            None => vec![0u8; size],
        }
    }

    /// Release a buffer back to the pool
    ///
    /// If a buffer pool is configured, returns the buffer to the pool for reuse.
    /// Otherwise, the buffer is dropped.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to release
    ///
    /// # Example
    ///
    /// ```
    /// use wraith_transport::worker::{WorkerPool, WorkerConfig};
    ///
    /// let config = WorkerConfig::with_buffer_pool(1500, 128);
    /// let pool = WorkerPool::new(config);
    ///
    /// let buffer = pool.acquire_buffer(1500);
    /// // ... use buffer ...
    /// pool.release_buffer(buffer);
    /// ```
    pub fn release_buffer(&self, buffer: Vec<u8>) {
        if let Some(pool) = &self.buffer_pool {
            pool.release(buffer);
        }
        // Otherwise, buffer is dropped
    }

    /// Get the buffer pool if configured
    ///
    /// Returns a clone of the buffer pool handle, or `None` if no pool is configured.
    pub fn buffer_pool(&self) -> Option<BufferPool> {
        self.buffer_pool.clone()
    }

    /// Submit a task to the worker pool
    ///
    /// # Errors
    /// Returns an error if the queue is full or the pool is shutting down.
    ///
    /// # Examples
    /// ```no_run
    /// # use wraith_transport::worker::{WorkerPool, WorkerConfig, Task};
    /// # let pool = WorkerPool::new(WorkerConfig::default());
    /// let task = Task::ProcessPacket {
    ///     data: vec![1, 2, 3],
    ///     source: 0,
    /// };
    /// pool.submit(task).unwrap();
    /// ```
    pub fn submit(&self, task: Task) -> Result<(), WorkerError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(WorkerError::ShuttingDown);
        }

        self.task_tx
            .try_send(task)
            .map_err(|_| WorkerError::QueueFull)
    }

    /// Get pool statistics
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Get the number of workers in the pool
    pub fn num_workers(&self) -> usize {
        self.workers.len()
    }

    /// Initiate graceful shutdown of all workers
    ///
    /// This method signals all workers to shut down and waits for them to finish.
    pub fn shutdown(self) {
        info!(
            "Shutting down worker pool with {} workers",
            self.workers.len()
        );
        self.shutdown.store(true, Ordering::Release);

        // Send shutdown signals to all workers
        for _ in 0..self.workers.len() {
            let _ = self.task_tx.send(Task::Shutdown);
        }

        // Wait for all workers to finish
        for worker in self.workers {
            if let Err(e) = worker.handle.join() {
                error!("Worker {} failed to join: {:?}", worker.id, e);
            }
        }

        info!("Worker pool shutdown complete");
    }
}

/// Individual worker thread
struct Worker {
    id: usize,
    handle: JoinHandle<()>,
}

impl Worker {
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
    fn spawn(
        id: usize,
        task_rx: Receiver<Task>,
        shutdown: Arc<AtomicBool>,
        stats: Arc<WorkerStats>,
        pin_to_cpu: bool,
        numa_aware: bool,
        buffer_pool: Option<BufferPool>,
    ) -> Self {
        let handle = thread::Builder::new()
            .name(format!("wraith-worker-{id}"))
            .spawn(move || {
                debug!("Worker {} starting", id);

                // Pin to CPU core if enabled
                #[cfg(target_os = "linux")]
                if pin_to_cpu {
                    if let Err(e) = Self::pin_to_cpu(id) {
                        warn!("Failed to pin worker {} to CPU: {}", id, e);
                    } else {
                        debug!("Worker {} pinned to CPU {}", id, id);
                    }
                }

                // Set up NUMA-aware allocation if enabled
                #[cfg(target_os = "linux")]
                if numa_aware && let Some(node) = crate::numa::get_numa_node_for_cpu(id) {
                    debug!("Worker {} on NUMA node {}", id, node);
                }

                // Worker event loop
                while !shutdown.load(Ordering::Acquire) {
                    match task_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(task) => {
                            stats.tasks_processed.fetch_add(1, Ordering::Relaxed);

                            match task {
                                Task::ProcessPacket { data, source } => {
                                    Self::process_packet(&data, source, &stats);
                                    // Release buffer back to pool if configured
                                    if let Some(ref pool) = buffer_pool {
                                        pool.release(data);
                                    }
                                }
                                Task::SendPacket { data, destination } => {
                                    Self::send_packet(&data, destination, &stats);
                                    // Release buffer back to pool if configured
                                    if let Some(ref pool) = buffer_pool {
                                        pool.release(data);
                                    }
                                }
                                Task::Shutdown => {
                                    debug!("Worker {} received shutdown signal", id);
                                    break;
                                }
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            warn!("Worker {} task channel disconnected", id);
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            // No task available, continue loop
                        }
                    }
                }

                info!(
                    "Worker {} shutting down (processed {} tasks, {} packets, {} bytes)",
                    id,
                    stats.tasks_processed.load(Ordering::Relaxed),
                    stats.packets_processed.load(Ordering::Relaxed),
                    stats.bytes_processed.load(Ordering::Relaxed)
                );
            })
            .expect("Failed to spawn worker thread");

        Self { id, handle }
    }

    #[cfg(target_os = "linux")]
    fn pin_to_cpu(core_id: usize) -> Result<(), String> {
        use std::mem;

        // SAFETY: sched_setaffinity is a standard Linux syscall. cpu_set_t is properly
        // zero-initialized via mem::zeroed(), and CPU_ZERO/CPU_SET are standard libc macros.
        // Passing 0 for pid means current thread, and size is correct for cpu_set_t.
        unsafe {
            let mut cpuset: libc::cpu_set_t = mem::zeroed();
            libc::CPU_ZERO(&mut cpuset);
            libc::CPU_SET(core_id, &mut cpuset);

            let ret = libc::sched_setaffinity(
                0, // Current thread
                mem::size_of::<libc::cpu_set_t>(),
                &cpuset,
            );

            if ret != 0 {
                Err(format!("sched_setaffinity failed with code {ret}"))
            } else {
                Ok(())
            }
        }
    }

    fn process_packet(data: &[u8], _source: usize, stats: &WorkerStats) {
        // Placeholder for packet processing logic
        // In a real implementation, this would:
        // 1. Decrypt the packet
        // 2. Parse the frame
        // 3. Handle the frame based on type
        // 4. Update session state

        stats.packets_processed.fetch_add(1, Ordering::Relaxed);
        stats
            .bytes_processed
            .fetch_add(data.len() as u64, Ordering::Relaxed);
    }

    fn send_packet(data: &[u8], _destination: usize, stats: &WorkerStats) {
        // Placeholder for packet sending logic
        // In a real implementation, this would:
        // 1. Frame the data
        // 2. Encrypt the frame
        // 3. Send via transport layer

        stats.packets_processed.fetch_add(1, Ordering::Relaxed);
        stats
            .bytes_processed
            .fetch_add(data.len() as u64, Ordering::Relaxed);
    }
}

/// Worker pool errors
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// Task queue is full
    #[error("Task queue is full")]
    QueueFull,

    /// Pool is shutting down
    #[error("Worker pool is shutting down")]
    ShuttingDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_default() {
        let config = WorkerConfig::default();
        assert_eq!(config.num_workers, 0); // Auto-detect
        assert_eq!(config.queue_capacity, 10000);
        assert!(config.buffer_pool.is_none());
    }

    #[test]
    fn test_worker_config_with_buffer_pool() {
        let config = WorkerConfig::with_buffer_pool(1500, 128);
        assert!(config.buffer_pool.is_some());
        let pool = config.buffer_pool.as_ref().unwrap();
        assert_eq!(pool.buffer_size(), 1500);
        assert_eq!(pool.capacity(), 128);
    }

    #[test]
    fn test_worker_pool_creation() {
        let config = WorkerConfig {
            num_workers: 2,
            queue_capacity: 100,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);
        assert_eq!(pool.num_workers(), 2);
    }

    #[test]
    fn test_worker_pool_submit_task() {
        let config = WorkerConfig {
            num_workers: 2,
            queue_capacity: 10,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);

        let task = Task::ProcessPacket {
            data: vec![1, 2, 3, 4],
            source: 0,
        };

        pool.submit(task).unwrap();

        // Give workers time to process
        std::thread::sleep(Duration::from_millis(50));

        let stats = pool.stats();
        assert!(stats.total_tasks() > 0);
    }

    #[test]
    fn test_worker_pool_shutdown() {
        let config = WorkerConfig {
            num_workers: 2,
            queue_capacity: 10,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);

        // Submit some tasks
        for i in 0..5 {
            let task = Task::ProcessPacket {
                data: vec![i; 100],
                source: 0,
            };
            pool.submit(task).unwrap();
        }

        // Give workers time to process
        std::thread::sleep(Duration::from_millis(50));

        // Shutdown should complete without hanging
        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_stats() {
        let config = WorkerConfig {
            num_workers: 2,
            queue_capacity: 100,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);

        // Submit tasks
        for i in 0..10 {
            let task = Task::ProcessPacket {
                data: vec![0; 100],
                source: i,
            };
            pool.submit(task).unwrap();
        }

        // Give workers time to process
        std::thread::sleep(Duration::from_millis(100));

        let stats = pool.stats();
        assert!(stats.total_tasks() > 0);
        assert!(stats.total_packets() > 0);
        assert!(stats.total_bytes() >= 1000); // At least 10 * 100 bytes
    }

    #[test]
    fn test_worker_pool_queue_full() {
        let config = WorkerConfig {
            num_workers: 1,
            queue_capacity: 5,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);

        let mut successes = 0;
        let mut failures = 0;

        // Fill the queue
        for i in 0..20 {
            let task = Task::ProcessPacket {
                data: vec![0; 10], // Smaller packets to process quickly
                source: i,
            };

            match pool.submit(task) {
                Ok(_) => successes += 1,
                Err(_) => failures += 1,
            }
        }

        // Should have at least some successes and some failures
        assert!(
            successes > 0,
            "Should have submitted some tasks successfully"
        );
        assert!(
            failures > 0,
            "Should have rejected some tasks when queue is full"
        );

        // Give workers time to process
        std::thread::sleep(Duration::from_millis(200));

        // Verify tasks were processed
        let stats = pool.stats();
        assert!(
            stats.total_tasks() > 0,
            "Workers should have processed tasks"
        );
    }

    #[test]
    fn test_worker_pool_auto_detect_workers() {
        let config = WorkerConfig {
            num_workers: 0, // Auto-detect
            ..Default::default()
        };

        let pool = WorkerPool::new(config);
        let num_cpus = num_cpus::get();
        assert_eq!(pool.num_workers(), num_cpus);
    }

    #[test]
    fn test_worker_pool_with_buffer_pool() {
        // Create pool with buffer pool
        let config = WorkerConfig {
            num_workers: 2,
            queue_capacity: 100,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: Some(BufferPool::new(1024, 64)),
        };

        let pool = WorkerPool::new(config);

        // Verify buffer pool is accessible
        assert!(pool.buffer_pool().is_some());
        let bp = pool.buffer_pool().unwrap();
        assert_eq!(bp.buffer_size(), 1024);

        // Acquire buffer via pool
        let buffer = pool.acquire_buffer(1024);
        assert_eq!(buffer.len(), 1024);

        // Submit task with buffer from pool
        let task = Task::ProcessPacket {
            data: buffer,
            source: 0,
        };
        pool.submit(task).unwrap();

        // Give workers time to process and release buffer
        std::thread::sleep(Duration::from_millis(100));

        // Buffer should be back in pool
        let stats = pool.stats();
        assert!(stats.total_tasks() > 0);

        pool.shutdown();
    }

    #[test]
    fn test_worker_pool_buffer_recycling() {
        let buffer_pool = BufferPool::new(256, 10);
        let config = WorkerConfig {
            num_workers: 1,
            queue_capacity: 50,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: Some(buffer_pool.clone()),
        };

        let pool = WorkerPool::new(config);

        // Initially, all 10 buffers should be available
        assert_eq!(buffer_pool.available(), 10);

        // Submit 5 tasks using buffers from the pool
        for i in 0..5 {
            let buf = buffer_pool.acquire();
            let task = Task::ProcessPacket {
                data: buf,
                source: i,
            };
            pool.submit(task).unwrap();
        }

        // Pool should have 5 less immediately after acquire
        assert_eq!(buffer_pool.available(), 5);

        // Give workers time to process and release buffers
        std::thread::sleep(Duration::from_millis(200));

        // All buffers should be recycled back
        assert_eq!(buffer_pool.available(), 10);

        pool.shutdown();
    }

    #[test]
    fn test_acquire_buffer_without_pool() {
        let config = WorkerConfig::default();
        let pool = WorkerPool::new(config);

        // Without buffer pool, should allocate new buffer
        let buffer = pool.acquire_buffer(512);
        assert_eq!(buffer.len(), 512);

        // Release should be a no-op (buffer is dropped)
        pool.release_buffer(buffer);
    }

    #[test]
    fn test_worker_stats_accumulation() {
        let stats = WorkerStats::default();

        stats.tasks_processed.fetch_add(5, Ordering::Relaxed);
        stats.packets_processed.fetch_add(10, Ordering::Relaxed);
        stats.bytes_processed.fetch_add(1000, Ordering::Relaxed);
        stats.errors.fetch_add(2, Ordering::Relaxed);

        assert_eq!(stats.tasks_processed.load(Ordering::Relaxed), 5);
        assert_eq!(stats.packets_processed.load(Ordering::Relaxed), 10);
        assert_eq!(stats.bytes_processed.load(Ordering::Relaxed), 1000);
        assert_eq!(stats.errors.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_pool_stats_aggregation() {
        let worker1 = Arc::new(WorkerStats::default());
        let worker2 = Arc::new(WorkerStats::default());

        worker1.tasks_processed.store(10, Ordering::Relaxed);
        worker1.packets_processed.store(20, Ordering::Relaxed);
        worker1.bytes_processed.store(500, Ordering::Relaxed);

        worker2.tasks_processed.store(15, Ordering::Relaxed);
        worker2.packets_processed.store(30, Ordering::Relaxed);
        worker2.bytes_processed.store(750, Ordering::Relaxed);

        let pool_stats = PoolStats {
            workers: vec![worker1, worker2],
            start_time: Some(Instant::now()),
        };

        assert_eq!(pool_stats.total_tasks(), 25);
        assert_eq!(pool_stats.total_packets(), 50);
        assert_eq!(pool_stats.total_bytes(), 1250);
    }

    #[test]
    fn test_pool_stats_throughput() {
        let worker = Arc::new(WorkerStats::default());
        worker.packets_processed.store(1000, Ordering::Relaxed);
        worker.bytes_processed.store(1024000, Ordering::Relaxed);

        let pool_stats = PoolStats {
            workers: vec![worker],
            start_time: Some(Instant::now() - Duration::from_secs(1)),
        };

        // After 1 second
        std::thread::sleep(Duration::from_millis(10));

        let pps = pool_stats.packets_per_second();
        let bps = pool_stats.bytes_per_second();

        // Should be roughly 1000 packets/sec and 1024000 bytes/sec
        assert!(pps > 0.0);
        assert!(bps > 0.0);
    }

    #[test]
    fn test_pool_stats_no_start_time() {
        let pool_stats = PoolStats {
            workers: vec![],
            start_time: None,
        };

        assert_eq!(pool_stats.packets_per_second(), 0.0);
        assert_eq!(pool_stats.bytes_per_second(), 0.0);
    }

    #[test]
    fn test_worker_error_display() {
        let err = WorkerError::QueueFull;
        assert_eq!(err.to_string(), "Task queue is full");

        let err = WorkerError::ShuttingDown;
        assert_eq!(err.to_string(), "Worker pool is shutting down");
    }

    #[test]
    fn test_task_enum_variants() {
        let task1 = Task::ProcessPacket {
            data: vec![1, 2, 3],
            source: 0,
        };

        let task2 = Task::SendPacket {
            data: vec![4, 5, 6],
            destination: 1,
        };

        let task3 = Task::Shutdown;

        match task1 {
            Task::ProcessPacket { data, source } => {
                assert_eq!(data, vec![1, 2, 3]);
                assert_eq!(source, 0);
            }
            _ => panic!("Wrong task type"),
        }

        match task2 {
            Task::SendPacket { data, destination } => {
                assert_eq!(data, vec![4, 5, 6]);
                assert_eq!(destination, 1);
            }
            _ => panic!("Wrong task type"),
        }

        assert!(matches!(task3, Task::Shutdown));
    }

    #[test]
    fn test_worker_config_defaults() {
        let config = WorkerConfig::default();

        assert_eq!(config.num_workers, 0); // Auto-detect
        assert_eq!(config.queue_capacity, 10000);
        assert!(config.buffer_pool.is_none());

        #[cfg(target_os = "linux")]
        {
            assert!(config.pin_to_cpu);
            assert!(config.numa_aware);
        }

        #[cfg(not(target_os = "linux"))]
        {
            assert!(!config.pin_to_cpu);
            assert!(!config.numa_aware);
        }
    }

    #[test]
    fn test_worker_pool_submit_after_shutdown() {
        let config = WorkerConfig {
            num_workers: 1,
            queue_capacity: 10,
            pin_to_cpu: false,
            numa_aware: false,
            buffer_pool: None,
        };

        let pool = WorkerPool::new(config);
        pool.shutdown.store(true, Ordering::Release);

        let task = Task::ProcessPacket {
            data: vec![1, 2, 3],
            source: 0,
        };

        let result = pool.submit(task);
        assert!(matches!(result, Err(WorkerError::ShuttingDown)));
    }

    #[test]
    fn test_worker_stats_errors_tracking() {
        let stats = WorkerStats::default();

        for _ in 0..5 {
            stats.errors.fetch_add(1, Ordering::Relaxed);
        }

        assert_eq!(stats.errors.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn test_pool_stats_total_errors() {
        let worker1 = Arc::new(WorkerStats::default());
        let worker2 = Arc::new(WorkerStats::default());

        worker1.errors.store(5, Ordering::Relaxed);
        worker2.errors.store(3, Ordering::Relaxed);

        let pool_stats = PoolStats {
            workers: vec![worker1, worker2],
            start_time: Some(Instant::now()),
        };

        assert_eq!(pool_stats.total_errors(), 8);
    }
}
