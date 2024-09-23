mod conv;

use chrono_tz::{Asia::Seoul, Tz};
use dotenvy::dotenv;
use figment::providers::{Format, Serialized, Toml};
use figment::Figment;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::env;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time::sleep;

use conv::TomlDatetimeAsChronoDateTimeSeoul;

#[derive(Deserialize, Serialize, Debug)]
struct Reminder {
    before: u64,
    message: String,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
struct Config {
    to: String,

    #[serde_as(as = "Vec<TomlDatetimeAsChronoDateTimeSeoul>")]
    at: Vec<chrono::DateTime<Tz>>,

    reminders: Vec<Reminder>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            to: String::new(),
            at: Vec::new(),
            reminders: Vec::new(),
        }
    }
}

#[derive(Serialize, Debug)]
struct Body {
    content: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    dotenv().ok();

    let configs: Vec<Config> = std::fs::read_dir("schedules")
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            let config: Config = Figment::new()
                .merge(Serialized::defaults(Config::default()))
                .merge(Toml::file(&path))
                .extract()
                .expect("Failed to load configuration");
            info!("Configuration fetched: {:?}", path.file_name().unwrap());
            config
        })
        .collect();

    let mut tasks = JoinSet::new();

    let now = chrono::Utc::now().with_timezone(&Seoul).timestamp();
    for config in configs {
        let hook_url = env::var(&config.to).unwrap();
        let at: Vec<i64> = config.at.iter().map(|dt| dt.timestamp() - now).collect();

        for t in at {
            for reminder in &config.reminders {
                let left = t.checked_sub_unsigned(reminder.before).unwrap();
                if left < 0 {
                    debug!(
                        "Skipping the completed task: hook={} delta={}",
                        config.to, left
                    );
                    continue;
                }

                let to = config.to.clone();
                let url = hook_url.clone();
                let message = reminder.message.clone();
                tasks.spawn(async move {
                    info!("Spawned: hook={} delta={}", to, left);

                    sleep(Duration::from_secs(left.try_into().unwrap())).await;

                    let client = reqwest::Client::new();
                    let body = Body {
                        content: format!("@everyone {message}"),
                    };

                    client
                        .post(url.as_str())
                        .header("Content-Type", "application/json")
                        .body(serde_json::to_string(&body).unwrap())
                        .send()
                        .await
                        .unwrap();

                    info!("Message sent: hook={}", to);
                });
            }
        }
    }

    while let Some(_) = tasks.join_next().await {}
}
