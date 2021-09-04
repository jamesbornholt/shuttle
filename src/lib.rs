// #![deny(warnings, missing_debug_implementations, missing_docs)]

//! Shuttle is a library for testing concurrent Rust code, heavily inspired by [Loom][].
//!
//! Shuttle focuses on randomized testing, rather than the exhaustive testing that Loom offers. This
//! is a soundness—scalability trade-off: Shuttle is not sound (a passing Shuttle test does not
//! prove the code is correct), but it scales to much larger test cases than Loom. Empirically,
//! randomized testing is successful at finding most concurrency bugs, which tend not to be
//! adversarial.
//!
//! ## Testing concurrent code
//!
//! Consider this simple piece of concurrent code:
//!
//! ```no_run
//! use std::sync::{Arc, Mutex};
//! use std::thread;
//!
//! let lock = Arc::new(Mutex::new(0u64));
//! let lock2 = lock.clone();
//!
//! thread::spawn(move || {
//!     *lock.lock().unwrap() = 1;
//! });
//!
//! assert_eq!(0, *lock2.lock().unwrap());
//! ```
//!
//! There is an obvious race condition here: if the spawned thread runs before the assertion, the
//! assertion will fail. But writing a unit test that finds this execution is tricky. We could run
//! the test many times and try to "get lucky" by finding a failing execution, but that's not a very
//! reliable testing approach. Even if the test does fail, it will be difficult to debug: we won't
//! be able to easily catch the failure in a debugger, and every time we make a change, we will need
//! to run the test many times to decide whether we fixed the issue.
//!
//! ### Randomly testing concurrent code with Shuttle
//!
//! Shuttle avoids this issue by controlling the scheduling of each thread in the program, and
//! scheduling those threads *randomly*. By controlling the scheduling, Shuttle allows us to
//! reproduce failing tests deterministically. By using random scheduling, with appropriate
//! heuristics, Shuttle can still catch most (non-adversarial) concurrency bugs even though it is
//! not an exhaustive checker.
//!
//! A Shuttle version of the above test just wraps the test body in a call to Shuttle's
//! [check_random] function, and replaces the concurrency-related imports from `std` with imports
//! from `shuttle`:
//!
//! ```should_panic
//! use shuttle::sync::{Arc, Mutex};
//! use shuttle::thread;
//!
//! shuttle::check_random(|| {
//!     let lock = Arc::new(Mutex::new(0u64));
//!     let lock2 = lock.clone();
//!
//!     thread::spawn(move || {
//!         *lock.lock().unwrap() = 1;
//!     });
//!
//!     assert_eq!(0, *lock2.lock().unwrap());
//! }, 100);
//! ```
//!
//! This test detects the assertion failure with extremely high probability (over 99.9999%).
//!
//! ## Testing non-deterministic code
//!
//! Shuttle supports testing code that uses *data non-determinism* (random number generation). For
//! example, this test uses the [`rand`](https://crates.io/crates/rand) crate to generate a random
//! number:
//!
//! ```no_run
//! use rand::{thread_rng, Rng};
//!
//! let x = thread_rng().gen::<u64>();
//! assert_eq!(x % 10, 7);
//! ```
//!
//! Shuttle provides its own implementation of [`rand`] that is a drop-in replacement:
//!
//! ```should_panic
//! use shuttle::rand::{thread_rng, Rng};
//!
//! shuttle::check_random(|| {
//!     let x = thread_rng().gen::<u64>();
//!     assert_ne!(x % 10, 7);
//! }, 100);
//! ```
//!
//! This test will run the body 100 times, and fail if any of those executions fails; the test
//! therefore fails with probability 1-(9/10)^100, or 99.997%. We can increase the `100` parameter
//! to run more executions and increase the probability of finding the failure. Note that Shuttle
//! isn't doing anything special to increase the probability of this test failing other than running
//! the body multiple times.
//!
//! When this test fails, Shuttle provides output that can be used to **deterministically**
//! reproduce the failure:
//!
//! ```text
//! test panicked in task "task-0" with schedule: "910102ccdedf9592aba2afd70104"
//! pass that schedule string into `shuttle::replay` to reproduce the failure
//! ```
//!
//! We can use Shuttle's [`replay`] function to replay the execution that causes the failure:
//!
//! ```should_panic
//! # // *** DON'T FORGET TO UPDATE THE TEXT OUTPUT RIGHT ABOVE THIS IF YOU CHANGE THIS TEST! ***
//! use shuttle::rand::{thread_rng, Rng};
//!
//! shuttle::replay(|| {
//!     let x = thread_rng().gen::<u64>();
//!     assert_ne!(x % 10, 7);
//! }, "910102ccdedf9592aba2afd70104");
//! ```
//!
//! This runs the test only once, and is guaranteed to reproduce the failure.
//!
//! Support for data non-determinism is most useful when *combined* with support for schedule
//! non-determinism (i.e., concurrency). For example, an integration test might spawn several
//! threads, and within each thread perform a random sequence of actions determined by `thread_rng`
//! (this style of testing is often referred to as a "stress test"). By using Shuttle to implement
//! the stress test, we can both increase the coverage of the test by exploring more thread
//! interleavings and allow test failures to be deterministically reproducible for debugging.
//!
//! ## Writing Shuttle tests
//!
//! To test concurrent code with Shuttle, all uses of synchronization primitives from `std` must be
//! replaced by their Shuttle equivalents. The simplest way to do this is via `cfg` flags.
//! Specifically, if you enforce that all synchronization primitives are imported from a single
//! `sync` module in your code, and implement that module like this:
//!
//! ```
//! #[cfg(all(feature = "shuttle", test))]
//! use shuttle::{sync::*, thread};
//! #[cfg(not(all(feature = "shuttle", test)))]
//! use std::{sync::*, thread};
//! ```
//!
//! Then a Shuttle test can be written like this:
//!
//! ```
//! # mod my_crate {}
//! #[cfg(feature = "shuttle")]
//! #[test]
//! fn concurrency_test_shuttle() {
//!     use my_crate::*;
//!     // ...
//! }
//! ```
//!
//! and be executed by running `cargo test --features shuttle`.
//!
//! ### Choosing a scheduler and running a test
//!
//! Shuttle tests need to choose a *scheduler* to use to direct the execution. The scheduler
//! determines the order in which threads are scheduled. Different scheduling policies can increase
//! the probability of detecting certain classes of bugs (e.g., race conditions), but at the cost of
//! needing to test more executions.
//!
//! Shuttle has a number of built-in schedulers, which implement the
//! [`Scheduler`](crate::scheduler::Scheduler) trait. They are most easily accessed via convenience
//! methods:
//! - [`check_random`] runs a test using a random scheduler for a chosen number of executions.
//! - [`check_pct`] runs a test using the [Probabilistic Concurrency Testing][pct] (PCT) algorithm.
//!   PCT bounds the number of preemptions a test explores; empirically, most concurrency bugs can
//!   be detected with very few preemptions, and so PCT increases the probability of finding such
//!   bugs. The PCT scheduler can be configured with a "bug depth" (the number of preemptions) and a
//!   number of executions.
//! - [`check_dfs`] runs a test with an *exhaustive* scheduler using depth-first search. Exhaustive
//!   testing is intractable for all but the very simplest programs, and so using this scheduler is
//!   not recommended, but it can be useful to thoroughly test small concurrency primitives. The DFS
//!   scheduler can be configured with a bound on the depth of schedules to explore.
//!
//! When these convenience methods do not provide enough control, Shuttle provides a [`Runner`]
//! object for executing a test. A runner is constructed from a chosen [scheduler](scheduler), and
//! then invoked with the [`Runner::run`] method. Shuttle also provides a [`PortfolioRunner`] object
//! for running multiple schedulers, using parallelism to increase the number of test executions
//! explored.
//!
//! [Loom]: https://github.com/tokio-rs/loom
//! [pct]: https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/asplos277-pct.pdf

pub mod asynch;
pub mod rand;
pub mod sync;
pub mod thread;

pub mod scheduler;

mod runtime;

pub use runtime::runner::{PortfolioRunner, Runner};

/// Configuration parameters for Shuttle
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Config {
    /// Stack size allocated for each thread
    pub stack_size: usize,

    /// How to persist schedules when a test fails
    pub failure_persistence: FailurePersistence,

    /// Maximum number of steps a single iteration of a test can take, and how to react when the
    /// limit is reached
    pub max_steps: MaxSteps,

    /// Time limit for an entire test. If set, calls to [`Runner::run`] will return when the time
    /// limit is exceeded or the [`Scheduler`](crate::scheduler::Scheduler) chooses to stop (e.g.,
    /// by hitting its maximum number of iterations), whichever comes first. This time limit will
    /// not abort a currently running test iteration; the limit is only checked between iterations.
    pub max_time: Option<std::time::Duration>,

    /// Whether to enable warnings about [Shuttle's unsound implementation of
    /// `atomic`](crate::sync::atomic#warning-about-relaxed-behaviors).
    pub silence_atomic_ordering_warning: bool,
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self {
            stack_size: 0x8000,
            failure_persistence: FailurePersistence::Print,
            max_steps: MaxSteps::FailAfter(1_000_000),
            max_time: None,
            silence_atomic_ordering_warning: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Specifies how to persist schedules when a Shuttle test fails
///
/// By default, schedules are printed to stdout/stderr, and can be replayed using [`replay`].
/// Optionally, they can instead be persisted to a file and replayed using [`replay_from_file`],
/// which can be useful if the schedule is too large to conveniently include in a call to
/// [`replay`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FailurePersistence {
    /// Do not persist failing schedules
    None,
    /// Print failing schedules to stdout/stderr
    Print,
    /// Persist schedules as files in the given directory, or the current directory if None.
    File(Option<std::path::PathBuf>),
}

/// Specifies an upper bound on the number of steps a single iteration of a Shuttle test can take,
/// and how to react when the bound is reached.
///
/// A "step" is an atomic region (all the code between two yieldpoints). For example, all the
/// (non-concurrency-operation) code between acquiring and releasing a [`Mutex`] is a single step.
/// Shuttle can bound the maximum number of steps a single test iteration can take to prevent
/// infinite loops. If the bound is hit, the test can either fail (`FailAfter`) or continue to the
/// next iteration (`ContinueAfter`).
///
/// The steps bound can be used to protect against livelock and fairness issues. For example, if a
/// thread is waiting for another thread to make progress, but the chosen [`Scheduler`] never
/// schedules that thread, a livelock occurs and the test will not terminate without a step bound.
///
/// By default, Shuttle fails a test after 1,000,000 steps.
///
/// [`Mutex`]: crate::sync::Mutex
/// [`Scheduler`]: crate::scheduler::Scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MaxSteps {
    /// Do not enforce any bound on the maximum number of steps
    None,
    /// Fail the test (by panicing) after the given number of steps
    FailAfter(usize),
    /// When the given number of steps is reached, stop the current iteration of the test and
    /// begin a new iteration
    ContinueAfter(usize),
}

/// Run the given function once under a round-robin concurrency scheduler.
// TODO consider removing this -- round robin scheduling is never what you want.
#[doc(hidden)]
pub fn check<F>(f: F)
where
    F: Fn() + Send + Sync + 'static,
{
    use crate::scheduler::RoundRobinScheduler;

    let runner = Runner::new(RoundRobinScheduler::new(), Default::default());
    runner.run(f);
}

/// Run the given function under a randomized concurrency scheduler for some number of iterations.
/// Each iteration will run a (potentially) different randomized schedule.
pub fn check_random<F>(f: F, iterations: usize)
where
    F: Fn() + Send + Sync + 'static,
{
    use crate::scheduler::RandomScheduler;

    let scheduler = RandomScheduler::new(iterations);
    let runner = Runner::new(scheduler, Default::default());
    runner.run(f);
}

/// Run the given function under a PCT concurrency scheduler for some number of iterations at the
/// given depth. Each iteration will run a (potentially) different randomized schedule.
pub fn check_pct<F>(f: F, iterations: usize, depth: usize)
where
    F: Fn() + Send + Sync + 'static,
{
    use crate::scheduler::PctScheduler;

    let scheduler = PctScheduler::new(depth, iterations);
    let runner = Runner::new(scheduler, Default::default());
    runner.run(f);
}

/// Run the given function under a depth-first-search scheduler until all interleavings have been
/// explored (but if the max_iterations bound is provided, stop after that many iterations).
pub fn check_dfs<F>(f: F, max_iterations: Option<usize>)
where
    F: Fn() + Send + Sync + 'static,
{
    use crate::scheduler::DfsScheduler;

    let scheduler = DfsScheduler::new(max_iterations, false);
    let runner = Runner::new(scheduler, Default::default());
    runner.run(f);
}

/// Run the given function according to a given encoded schedule, usually produced as the output of
/// a failing Shuttle test case.
///
/// This function allows deterministic replay of a failing schedule, as long as `f` contains no
/// non-determinism other than that introduced by scheduling.
///
/// This is a convenience function for constructing a [`Runner`] that uses
/// [`ReplayScheduler::new_from_encoded`](scheduler::ReplayScheduler::new_from_encoded).
pub fn replay<F>(f: F, encoded_schedule: &str)
where
    F: Fn() + Send + Sync + 'static,
{
    use crate::scheduler::ReplayScheduler;

    let scheduler = ReplayScheduler::new_from_encoded(encoded_schedule);
    let runner = Runner::new(scheduler, Default::default());
    runner.run(f);
}

/// Run the given function according to a schedule saved in the given file, usually produced as the
/// output of a failing Shuttle test case.
///
/// This function allows deterministic replay of a failing schedule, as long as `f` contains no
/// non-determinism other than that introduced by scheduling.
///
/// This is a convenience function for constructing a [`Runner`] that uses
/// [`ReplayScheduler::new_from_file`](scheduler::ReplayScheduler::new_from_file).
pub fn replay_from_file<F, P>(f: F, path: P)
where
    F: Fn() + Send + Sync + 'static,
    P: AsRef<std::path::Path>,
{
    use crate::scheduler::ReplayScheduler;

    let scheduler = ReplayScheduler::new_from_file(path).expect("could not load schedule from file");
    let runner = Runner::new(scheduler, Default::default());
    runner.run(f);
}

/// The number of context switches that happened so far in the current Shuttle execution.
///
/// Note that this is the number of *possible* context switches, i.e., including times when the
/// scheduler decided to continue with the same task.
///
/// Panics if called outside of a Shuttle execution.
pub fn context_switches() -> usize {
    crate::runtime::execution::ExecutionState::context_switches()
}

/// Gets the current thread's vector clock
pub fn my_clock() -> crate::runtime::task::clock::VectorClock {
    crate::runtime::execution::ExecutionState::with(|state| {
        let me = state.current();
        state.get_clock(me.id()).clone()
    })
}

/// Gets the clock for the thread with the given task_id
pub fn get_clock(task_id: crate::runtime::task::TaskId) -> crate::runtime::task::clock::VectorClock {
    crate::runtime::execution::ExecutionState::with(|state| state.get_clock(task_id).clone())
}

/// Declare a new thread local storage key of type [`LocalKey`](crate::thread::LocalKey).
#[macro_export]
macro_rules! thread_local {
    // empty (base case for the recursion)
    () => {};

    // process multiple declarations with a const initializer
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const { $init:expr }; $($rest:tt)*) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
        $crate::thread_local!($($rest)*);
    );

    // handle a single declaration with a const initializer
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const { $init:expr }) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
    );

    // process multiple declarations
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
        $crate::thread_local!($($rest)*);
    );

    // handle a single declaration
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $crate::__thread_local_inner!($(#[$attr])* $vis $name, $t, $init);
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __thread_local_inner {
    ($(#[$attr:meta])* $vis:vis $name:ident, $t:ty, $init:expr) => {
        $(#[$attr])* $vis const $name: $crate::thread::LocalKey<$t> =
            $crate::thread::LocalKey {
                init: || { $init },
                _p: std::marker::PhantomData,
            };
    }
}

/// TODO
pub fn model<F>(f: F)
where
    F: Fn() + Sync + Send + 'static,
{
    check_random(f, 10000)
}

pub mod model {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[derive(Debug, Default)]
    pub struct Builder {
        /// Max number of threads to check as part of the execution.
        ///
        /// This should be set as low as possible and must be less than
        /// [`MAX_THREADS`](crate::MAX_THREADS).
        pub max_threads: usize,

        /// Maximum number of thread switches per permutation.
        ///
        /// Defaults to `LOOM_MAX_BRANCHES` environment variable.
        pub max_branches: usize,

        /// Maximum number of permutations to explore.
        ///
        /// Defaults to `LOOM_MAX_PERMUTATIONS` environment variable.
        pub max_permutations: Option<usize>,

        /// Maximum amount of time to spend on checking
        ///
        /// Defaults to `LOOM_MAX_DURATION` environment variable.
        pub max_duration: Option<Duration>,

        /// Maximum number of thread preemptions to explore
        ///
        /// Defaults to `LOOM_MAX_PREEMPTIONS` environment variable.
        pub preemption_bound: Option<usize>,

        /// When doing an exhaustive check, uses the file to store and load the
        /// check progress
        ///
        /// Defaults to `LOOM_CHECKPOINT_FILE` environment variable.
        pub checkpoint_file: Option<PathBuf>,

        /// How often to write the checkpoint file
        ///
        /// Defaults to `LOOM_CHECKPOINT_INTERVAL` environment variable.
        pub checkpoint_interval: usize,

        /// When `true`, locations are captured on each loom operation.
        ///
        /// Note that is is **very** expensive. It is recommended to first isolate a
        /// failing iteration using `LOOM_CHECKPOINT_FILE`, then enable location
        /// tracking.
        ///
        /// Defaults to `LOOM_LOCATION` environment variable.
        pub location: bool,

        /// Log execution output to stdout.
        ///
        /// Defaults to existance of `LOOM_LOG` environment variable.
        pub log: bool,
    }

    impl Builder {
        pub fn new() -> Builder {
            Default::default()
        }

        pub fn check<F>(&self, f: F)
        where
            F: Fn() + Sync + Send + 'static,
        {
            check_random(f, 10000)
        }
    }
}
pub mod future {
    pub use super::asynch::*;
    use crate::sync::Mutex;
    use std::task::Waker;

    /// Mock implementation of `tokio::sync::AtomicWaker`.
    #[derive(Debug)]
    pub struct AtomicWaker {
        waker: Mutex<Option<Waker>>,
    }

    impl AtomicWaker {
        /// Create a new instance of `AtomicWaker`.
        pub fn new() -> AtomicWaker {
            AtomicWaker {
                waker: Mutex::new(None),
            }
        }

        /// Registers the current task to be notified on calls to `wake`.
        pub fn register(&self, waker: Waker) {
            match self.waker.try_lock() {
                Ok(mut lock) => {
                    *lock = Some(waker);
                }
                Err(_) => {
                    waker.wake();
                    crate::thread::yield_now();
                }
            }
        }

        /// Registers the current task to be woken without consuming the value.
        pub fn register_by_ref(&self, waker: &Waker) {
            self.register(waker.clone());
        }

        /// Notifies the task that last called `register`.
        pub fn wake(&self) {
            if let Some(waker) = self.take_waker() {
                waker.wake();
            }
        }

        /// Attempts to take the `Waker` value out of the `AtomicWaker` with the
        /// intention that the caller will wake the task later.
        pub fn take_waker(&self) -> Option<Waker> {
            self.waker.lock().unwrap().take()
        }
    }

    impl Default for AtomicWaker {
        fn default() -> Self {
            AtomicWaker::new()
        }
    }
}
