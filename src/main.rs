use std::sync::Arc;
use std::{env, fs, io};

use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};

use serenity::utils::Color;
use serenity::{async_trait, Error, model::{channel::Message, gateway::Ready}, prelude::*};
use serenity::model::id::GuildId;

use tokio_postgres::NoTls;

// Container for psql client
struct DataClient {
    _tokio_postgres: tokio_postgres::Client,
}

impl TypeMapKey for DataClient {
    type Value = Arc<tokio_postgres::Client>;
}

// General framework for commands
#[group]
#[commands(help)]
struct General;

struct MessageHandler;

#[async_trait]
impl EventHandler for MessageHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} online", ready.user.name);
    }
}

#[tokio::main]
async fn main() {

    let token = env::var("DISCORD_TOKEN")
        .expect("Discord token not found");

    // Database settings from environment variable.
    // Format: host= <> dbname= <> user= <> password= <>
    let db_connection_settings = env::var("DB_CONNECTION")
        .expect("Database connection string not found. Set environment variable!");

    let (db_client, db_connection) = tokio_postgres::connect(&db_connection_settings, NoTls)
        .await
        .expect("Connection to the database failed!");

    // moving database connection to its own thread
    tokio::spawn(async move {
        if let Err(e) = db_connection.await {
            eprintln!("Connection Error: {}", e);
        }
    });

    // Serenity framework
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("q!").case_insensitivity(true))
        .group(&GENERAL_GROUP);

    // Serenity discord client builder
    let mut discord_client = Client::builder(&token)
        .event_handler(MessageHandler)
        .framework(framework)
        .await
        .expect("Building discord client failed");

    // psql container Arc
    {
        let mut data = discord_client.data.write().await;
        data.insert::<DataClient>(Arc::new(db_client));
    }

    // Starting discord client
    if let Err(e) = discord_client.start().await {
        println!("Starting client error {}", e)
    }

 }

// Setting the channel id from the database for the server id in question
// guild_id is from parsed within the command.
// channel_id: String - Channel id to be set in the database
async fn set_channel_id(channel_id: String, guild_id: String, ctx: &Context) -> Result<u64, tokio_postgres::Error> {
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    // Assuming the channel ID is a valid one, parsed at command level
    // Upserting into the database
    let upsert = client
        .execute(
            "INSERT INTO channels (guild_id, channel_id)
            VALUES ($1, $2)
            ON CONFLICT (guild_id)
            DO
            UPDATE SET channel_id = EXCLUDED.channel_id",
            &[&guild_id, &channel_id],
        )
        .await;

    upsert
}

// Pulls channel id from the database using the guild id.
// Returns 0 if no result
async fn get_channel_id(guild_id: String, ctx: &Context) -> String {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let mut channel_id;
    let rows = client
        .query(
            "SELECT channel_id FROM channels WHERE guild_id = $1",
            &[&guild_id]
        )
        .await
        .expect("Error querying database");
    if rows.len() > 0 {
        channel_id = rows[0].get(0);
    }
    else {
        channel_id = String::from("0");
    }
    channel_id
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    println!("I'm help lmao");

    Ok(())
}

async fn set_qotd_channel(ctx: &Context, msg: &Message) -> CommandResult {


    Ok(())
}

