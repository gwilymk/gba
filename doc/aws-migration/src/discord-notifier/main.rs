use aws_lambda_events::sns::SnsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CloudWatchAlarm {
    alarm_name: String,
    new_state_value: String,
    new_state_reason: String,
    #[serde(default)]
    alarm_description: String,
    #[serde(default)]
    state_change_time: String,
    #[serde(default)]
    region: String,
}

#[derive(Serialize)]
struct DiscordWebhook {
    embeds: Vec<DiscordEmbed>,
}

#[derive(Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    footer: DiscordFooter,
}

#[derive(Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Serialize)]
struct DiscordFooter {
    text: String,
}

fn state_to_color(state: &str) -> u32 {
    match state {
        "ALARM" => 0xFF0000,             // Red
        "OK" => 0x00FF00,                // Green
        "INSUFFICIENT_DATA" => 0xFFFF00, // Yellow
        _ => 0x808080,                   // Gray
    }
}

fn state_to_emoji(state: &str) -> &'static str {
    match state {
        "ALARM" => "🚨",
        "OK" => "✅",
        "INSUFFICIENT_DATA" => "⚠️",
        _ => "❓",
    }
}

async fn handler(event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    let webhook_url = env::var("DISCORD_WEBHOOK_URL")?;
    let client = reqwest::Client::new();

    for record in event.payload.records {
        let message = record.sns.message;
        let alarm: CloudWatchAlarm = serde_json::from_str(&message)?;

        let emoji = state_to_emoji(&alarm.new_state_value);
        let color = state_to_color(&alarm.new_state_value);

        // Truncate reason to Discord's field limit
        let reason = if alarm.new_state_reason.len() > 1024 {
            format!("{}...", &alarm.new_state_reason[..1021])
        } else {
            alarm.new_state_reason
        };

        let embed = DiscordEmbed {
            title: format!("{} {}", emoji, alarm.alarm_name),
            description: alarm.alarm_description,
            color,
            fields: vec![
                DiscordField {
                    name: "State".to_string(),
                    value: alarm.new_state_value,
                    inline: true,
                },
                DiscordField {
                    name: "Region".to_string(),
                    value: if alarm.region.is_empty() {
                        "us-west-2".to_string()
                    } else {
                        alarm.region
                    },
                    inline: true,
                },
                DiscordField {
                    name: "Reason".to_string(),
                    value: reason,
                    inline: false,
                },
            ],
            timestamp: if alarm.state_change_time.is_empty() {
                None
            } else {
                Some(alarm.state_change_time)
            },
            footer: DiscordFooter {
                text: "AWS CloudWatch".to_string(),
            },
        };

        let webhook = DiscordWebhook {
            embeds: vec![embed],
        };

        let response = client.post(&webhook_url).json(&webhook).send().await?;

        if !response.status().is_success() {
            tracing::error!(
                "Discord webhook failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    run(service_fn(handler)).await
}
