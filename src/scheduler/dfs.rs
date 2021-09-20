use crate::runtime::task::TaskId;
use crate::scheduler::data::fixed::FixedDataSource;
use crate::scheduler::data::DataSource;
use crate::scheduler::{Schedule, Scheduler};

const DFS_RANDOM_SEED: u64 = 0x12345678;

/// A scheduler that performs an exhaustive, depth-first enumeration of all possible schedules.
#[derive(Debug)]
pub struct DfsScheduler {
    max_iterations: Option<usize>,
    max_preemptions: Option<usize>,
    allow_random_data: bool,

    iterations: usize,
    // Vec<(previous choice, was that the last choice at that level)>
    levels: Vec<(TaskId, bool)>,
    steps: usize,
    next_change_level: Option<usize>,
    preemptions: usize,

    data_source: FixedDataSource,
}

impl DfsScheduler {
    /// Construct a new depth-first enumerative scheduler, with optional bounds on how many
    /// iterations to run and how many preemptions to allow.
    ///
    /// A `DfsScheduler` can optionally allow random data to be generated by the test (using
    /// [`shuttle::rand`](crate::rand)). Enabling random data makes the DFS search incomplete, as it
    /// will not explore all possible values for the random choices. To ensure determinism, each
    /// execution will use the same sequence of random choices.
    pub fn new(max_iterations: Option<usize>, max_preemptions: Option<usize>, allow_random_data: bool) -> Self {
        let data_source = FixedDataSource::initialize(DFS_RANDOM_SEED);

        Self {
            max_iterations,
            max_preemptions,
            iterations: 0,
            levels: vec![],
            steps: 0,
            next_change_level: None,
            preemptions: 0,
            allow_random_data,
            data_source,
        }
    }
}

impl Scheduler for DfsScheduler {
    fn new_execution(&mut self) -> Option<Schedule> {
        if self.max_iterations.map(|mi| self.iterations >= mi).unwrap_or(false) {
            return None;
        }

        // Find the index at which we'll make the next change -- the highest index in `levels` where
        // there are more choices to be made
        self.next_change_level = self
            .levels
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (_, is_last))| !*is_last)
            .map(|(level, _)| level);

        if self.next_change_level.is_some() || self.iterations == 0 {
            self.iterations += 1;
            self.steps = 0;
            self.preemptions = 0;
            Some(Schedule::new(self.data_source.reinitialize()))
        } else {
            None
        }
    }

    // TODO should we respect `is_yielding` by not allowing `current` to be scheduled next? That
    // TODO would be unsound but perhaps useful for validating some code
    fn next_task(&mut self, runnable: &[TaskId], current: Option<TaskId>, _is_yielding: bool) -> Option<TaskId> {
        let next = if self.steps >= self.levels.len() {
            // First time we've reached this level
            debug_assert_eq!(self.steps, self.levels.len());

            let hit_max_preemptions = self.max_preemptions.map(|mp| self.preemptions >= mp).unwrap_or(false);
            let force_run_current =
                hit_max_preemptions && current.map(|current| runnable.contains(&current)).unwrap_or(false);

            let to_run = if force_run_current {
                current.expect("we checked that current is runnable")
            } else {
                *runnable.first().unwrap()
            };

            let was_last = hit_max_preemptions || runnable.len() == 1;
            self.levels.push((to_run, was_last));
            to_run
        } else {
            // We've been to this level before. Is it the level at which we chose to make a change
            // for this iteration?
            let (last_choice, was_last) = self.levels[self.steps];
            if self.next_change_level.map(|level| level == self.steps).unwrap_or(false) {
                debug_assert!(!was_last, "next_change_level must have more choices available");
                let next_idx = runnable.iter().position(|tid| *tid == last_choice).unwrap() + 1;
                let next = runnable[next_idx];
                self.levels.drain(self.steps..);
                // This code is reachable only if we already chose to make a change at this level,
                // which means we cannot have reached our preemption limit. So we don't need to
                // account for the preemption limit here when computing `was_last`.
                self.levels.push((next, next_idx == runnable.len() - 1));
                next
            } else {
                last_choice
            }
        };

        if current
            .map(|current| current != next && runnable.contains(&current))
            .unwrap_or(false)
        {
            self.preemptions += 1;
        }

        self.steps += 1;

        Some(next)
    }

    fn next_u64(&mut self) -> u64 {
        if !self.allow_random_data {
            panic!("requested random data from DFS scheduler with allow_random_data = false");
        }
        self.data_source.next_u64()
    }
}
