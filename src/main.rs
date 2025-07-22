use poise::serenity_prelude::{ChannelId, ClientBuilder, GatewayIntents};
use reqwest::Client;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use std::sync::LazyLock;
use tokio::sync::Mutex;
use tokio::time;

mod parse;

struct Data {
    client: Client,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const CHANNEL_ID: u64 = 1395424977502863471;
const INTERVAL_SECS: u64 = 300; // 5 minutes

// Had to use a global variable because the secrets can't be passed as a parameter
static SECRETS: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::new()));

/// Checks for any new announcements
#[poise::command(slash_command)]
async fn check(ctx: Context<'_>) -> Result<(), Error> {
    let client = ctx.data().client.clone();
    let message = parse::parse_xml(&SECRETS, client).await;
    ctx.say(message).await?;
    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .expect("Secret 'DISCORD_TOKEN' not found");

    for var in ["XML_FEED", "RESTDB_API_KEY", "RESTDB_DATABASE"] {
        if let Some(secret) = secret_store.get(var) {
            SECRETS.lock().await.push_str(&(secret + " "));
        } else {
            panic!("Secret '{var}' not found");
        }
    }

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![check()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let client = Client::new();
                let data = Data { client };

                let msg_ctx = ctx.clone();
                let client_ref = &data.client;
                tokio::spawn({
                    let client = client_ref.clone();
                    async move {
                        let channel_id = ChannelId::new(CHANNEL_ID);
                        loop {
                            let message = parse::parse_xml(&SECRETS, client.clone()).await;
                            if let Err(e) = channel_id.say(&msg_ctx, message).await {
                                eprintln!("Failed to send message: {e}");
                            }
                            time::sleep(time::Duration::from_secs(INTERVAL_SECS)).await;
                        }
                    }
                });

                Ok(data)
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, GatewayIntents::non_privileged())
        .framework(framework)
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    Ok(client.into())
}
