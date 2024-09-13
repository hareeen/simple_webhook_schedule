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
struct Hook {
    to: String,

    #[serde_as(as = "Vec<TomlDatetimeAsChronoDateTimeSeoul>")]
    at: Vec<chrono::DateTime<Tz>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    reminders: Vec<Reminder>,
    hooks: Vec<Hook>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            reminders: Vec::new(),
            hooks: Vec::new(),
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

    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("Config.toml"))
        .extract()
        .expect("Failed to load configuration");
    info!("Configuration fetched");

    let mut tasks = JoinSet::new();

    let now = chrono::Utc::now().with_timezone(&Seoul).timestamp();
    for hook in config.hooks {
        let hook_url = env::var(&hook.to).unwrap();
        let at: Vec<i64> = hook.at.iter().map(|dt| dt.timestamp() - now).collect();

        for t in at {
            for reminder in &config.reminders {
                let left = t.checked_sub_unsigned(reminder.before).unwrap();
                if left < 0 {
                    debug!(
                        "Skipping the completed task: hook={} delta={}",
                        hook.to, left
                    );
                    continue;
                }

                let to = hook.to.clone();
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
