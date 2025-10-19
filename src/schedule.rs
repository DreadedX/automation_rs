use std::collections::HashMap;
use std::pin::Pin;

use automation_lib::action_callback::ActionCallback;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

pub async fn start_scheduler(
    schedule: HashMap<String, ActionCallback<()>>,
) -> Result<(), JobSchedulerError> {
    let scheduler = JobScheduler::new().await?;

    for (s, f) in schedule {
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
