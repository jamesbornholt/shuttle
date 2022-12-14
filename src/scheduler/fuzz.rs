use crate::runtime::task::TaskId;
use crate::scheduler::{Schedule, Scheduler};
use crate::scheduler::data::random::RandomDataSource;
use crate::scheduler::data::DataSource;

#[derive(Debug)]
pub struct FuzzScheduler {
    // any number of iterations..?
    schedule: Schedule,
    // is the current schedule complete -- if no, we cannot reset the schedule yet
    complete: bool,
    // number of steps we have taken so far
    steps: usize,
    // i lowkey don't know what this is for
    data_source: RandomDataSource,
}

impl FuzzScheduler {
    // you can only make a fuzz scheduler by passing in a schedule
    // can choose to not pass in a schedule.
    // need to figure out how to pass in schedule on creation
    pub fn new(schedule: Schedule) -> Self {
        Self {
            schedule: schedule,
            complete: false,
            steps: 0,
            data_source: RandomDataSource::initialize(0),
        }
    }

    pub fn new_execution_fuzz(&mut self, schedule: Schedule) -> Option<Schedule> {
        //generate new fuzz schedule (need a schedule to be passed in)
        //TODO: make sure that the previous schedule has run to completion
        if self.complete {
            self.complete = false;
            self.schedule = schedule;
        }

        // always return current schedule... not sure if best idea
        return Some(schedule);
    }

}

// TODO: double check that you can only run one schedule at a time
impl Scheduler for FuzzScheduler {
    fn new_execution(&mut self) -> Option<Schedule> {
        // lol idk just don't call this smh
        eprintln!(
            "incorrect usage of fuzz scheduler"
        );
        None
    }
    fn next_task( &mut self,
        runnable_tasks: &[TaskId],
        current_task: Option<TaskId>,
        is_yielding: bool,
    ) -> Option<TaskId>{

        // not sure if the commented code is necessary..?
        // if self.steps >= self.schedule.steps.len() {
        //     assert!(self.allow_incomplete, "schedule ended early");
        //     return None;
        // }

        // TODO: just stole this from replay. may not even be right, not sure
        match self.schedule.steps[self.steps] {
            ScheduleStep::Random => {
                panic!("can't do anything with random choice -- supposed to be guided.");
            }
            ScheduleStep::Task(next) => {
                if !runnable.contains(&next) {
                    assert!(
                        self.allow_incomplete,
                        "scheduled task is not runnable, expected to run {:?}, but choices were {:?}",
                        next, runnable
                    );
                    None
                } else {
                    self.steps += 1;
                    Some(next)
                }
            }
        }

    }

    fn next_u64(&mut self) -> u64 {
        self.data_source.next_u64()
    }

}

