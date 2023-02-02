use crate::runtime::task::TaskId;
use crate::scheduler::{Schedule, ScheduleStep, Scheduler};
use crate::scheduler::data::random::RandomDataSource;
use crate::scheduler::data::DataSource;

#[derive(Debug)]
pub struct FuzzScheduler {
    // any number of iterations..?
    // this doesn't need to be necessary anymore
    schedule: Option<Schedule>,
    // is the current schedule complete -- if no, we cannot reset the schedule yet
    complete: bool,
    // number of steps we have taken so far
    steps: usize,
    // i lowkey don't know what this is for
    data_source: RandomDataSource,
}

impl FuzzScheduler {
    pub fn new() -> Self {
        Self {
            schedule: Some(Schedule::new(0)),
            // this is just a placeholder, so you should be free to change it
            complete: true,
            steps: 0,
            data_source: RandomDataSource::initialize(0),
        }
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
        //make sure these are/are not needed?
        _current_task: Option<TaskId>,
        _is_yielding: bool,
    ) -> Option<TaskId>{

        // not sure if the commented code is necessary..?
        // if self.steps >= self.schedule.steps.len() {
        //     assert!(self.allow_incomplete, "schedule ended early");
        //     return None;
        // }
        match &self.schedule {
            Some(schedule) => {
                // TODO: just stole this from replay. may not even be right, not sure
                println!("BBBBBBBBBBO {0:?}, {1}", schedule.steps, self.steps);
                if schedule.steps.len() <= self.steps {
                    println!("in that thicc thicc if statement");
                    if runnable_tasks.len() > 0 {
                        return Some(runnable_tasks[0]);
                    }
                    else {
                        return None;
                    }
                }
                match schedule.steps[self.steps] {
                    ScheduleStep::Random => {
                        panic!("can't do anything with random choice -- supposed to be guided.");
                    }
                    ScheduleStep::Task(next) => {
                        if !runnable_tasks.contains(&next) {
                            // we have generated an incorrect schedule
                            None
                        } else {
                            self.steps += 1;
                            if self.steps >= schedule.steps.len(){
                                // we have completed the thing
                                self.complete = true;
                            }
                            Some(next)
                        }
                    }
                }
            }
            None => {
                eprintln!("incorrect use of fuzz scheduler");
                None
            }
        }

        

    }

    fn next_u64(&mut self) -> u64 {
        self.data_source.next_u64()
    }

    fn new_execution_fuzz(&mut self, schedule: Option<Schedule>) -> Option<Schedule> {
        //generate new fuzz schedule (need a schedule to be passed in)
        //TODO: make sure that the previous schedule has run to completion

        if self.complete {
            self.steps = 0;
            self.complete = false;
            self.schedule = schedule;
            self.schedule.clone()
        }
        else {
            //couldn't reset schedule
            None
        }

        // always return current schedule... not sure if best idea
        
    }

}

