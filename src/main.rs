use chrono::NaiveDateTime;
use chrono_tz::Asia::Seoul;
use dotenvy::dotenv;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::env;
use std::time::Duration;

use tokio::{
    task::JoinSet,
    time::{self, sleep},
};

#[derive(Serialize, Deserialize, Debug)]
struct Reminder {
    before: u64,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hook {
    to: String,
    at: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    reminders: Vec<Reminder>,
    hooks: Vec<Hook>,
}

#[derive(Serialize, Debug)]
struct Body {
    content: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    dotenv().expect(".env file not found");
    info!("Environment file loaded");

    let config_url = env::var("CONFIG_URL").unwrap();

    let mut tasks = JoinSet::new();

    let mut interval = time::interval(Duration::from_secs(60 * 60));
    loop {
        interval.tick().await;
        info!("Timer tick occurred");

        let resp = reqwest::get(&config_url)
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let config: Config = from_str(resp.as_str()).unwrap();
        info!("Configuration fetched");

        info!("Refreshing; count={}", tasks.len());

        tasks.shutdown().await;
        info!("Aborted all tasks");

        let now = chrono::Utc::now().with_timezone(&Seoul).timestamp();
        for hook in config.hooks {
            let hook_url = env::var(&hook.to).unwrap();
            let at: Vec<i64> = hook
                .at
                .iter()
                .map(|s| {
                    NaiveDateTime::parse_from_str(s, "%Y %m %d %H %M")
                        .unwrap()
                        .and_local_timezone(Seoul)
                        .unwrap()
                        .timestamp()
                        - now
                })
                .collect();

            for t in at {
                for reminder in &config.reminders {
                    let left = t.checked_sub_unsigned(reminder.before).unwrap();
                    if left < 0 {
                        warn!("Skipping the completed task; hook={} delta={}", hook.to, left);
                        continue;
                    }

                    let to = hook.to.clone();
                    let url = hook_url.clone();
                    let message = reminder.message.clone();
                    tasks.spawn(async move {
                        info!("Spawned; hook={} delta={}", to, left);

                        sleep(Duration::from_secs(left.try_into().unwrap())).await;

                        let client = reqwest::Client::new();
                        let body = Body {
                            content: format!("@everyone {message}"),
                        };

                        client
                            .post(url.as_str())
                            .header("Content-Type", "application/json")
                            .body(to_string(&body).unwrap())
                            .send()
                            .await
                            .unwrap();

                        info!("Message sent; hook={}", to);
                    });
                }
            }

            info!("Refreshed; count={}", tasks.len());
        }
    }
}
