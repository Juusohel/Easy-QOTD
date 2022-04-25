use std::sync::Arc;
use std::{env, fs};

use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};

use serenity::utils::Color;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
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


#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    println!("I'm help lmao");

    Ok(())
}

async fn set_qotd_channel(ctx: &Context, msg: &Message) -> CommandResult {


    Ok(())
}

