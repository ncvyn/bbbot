use poise::serenity_prelude::{ChannelId, ClientBuilder, CreateMessage, GatewayIntents};
use reqwest::Client;
use shuttle_runtime::SecretStore;
use shuttle_serenity::ShuttleSerenity;
use tokio::time;

mod parse;

struct Data {
    client: Client,
    secrets: String,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

const CHANNEL_ID: u64 = 1395424977502863471;
const INTERVAL_SECS: u64 = 300; // 5 minutes

/// Checks for any new announcements
#[poise::command(slash_command)]
async fn check(ctx: Context<'_>) -> Result<(), Error> {
    let secrets = &ctx.data().secrets;
    let client = ctx.data().client.clone();

    let embeds = parse::parse_xml(secrets, client).await;
    if embeds.is_empty() {
        return Ok(());
    }

    let builder = CreateMessage::new().embeds(embeds);
    let channel_id = ChannelId::new(CHANNEL_ID);
    channel_id.send_message(&ctx, builder).await?;
    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .expect("Secret 'DISCORD_TOKEN' not found");

    let mut secrets: String = String::new();

    for var in ["XML_FEED", "RESTDB_API_KEY", "RESTDB_DATABASE"] {
        if let Some(secret) = secret_store.get(var) {
            secrets.push_str(&(secret + " "));
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
                let data = Data { client, secrets };

                let msg_ctx = ctx.clone();

                tokio::spawn({
                    let client = data.client.clone();
                    let secrets = data.secrets.clone();

                    async move {
                        let channel_id = ChannelId::new(CHANNEL_ID);

                        loop {
                            let embeds = parse::parse_xml(&secrets, client.clone()).await;
                            let builder = CreateMessage::new().embeds(embeds);

                            if let Err(e) = channel_id.send_message(&msg_ctx, builder).await {
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
