use std::pin::Pin;

use automation_lib::action_callback::ActionCallback;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

#[derive(Debug, Default)]
pub struct Scheduler {
    jobs: Vec<(String, ActionCallback<()>)>,
}

impl Scheduler {
    pub fn add_job(&mut self, cron: String, f: ActionCallback<()>) {
        self.jobs.push((cron, f));
    }

    pub async fn start(self) -> Result<(), JobSchedulerError> {
        let scheduler = JobScheduler::new().await?;

        for (s, f) in self.jobs {
            let job = {
                move |_uuid, _lock| -> Pin<Box<dyn Future<Output = ()> + Send>> {
                    let f = f.clone();

                    Box::pin(async move {
                        f.call(()).await;
                    })
                }
            };

            let job = Job::new_async(s, job)?;

            scheduler.add(job).await?;
        }

        scheduler.start().await
    }
}
