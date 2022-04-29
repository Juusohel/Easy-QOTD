use std::sync::Arc;
use std::{env, fs, io};

use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};

use serenity::utils::{Color, parse_channel};
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
#[commands(help, set_qotd_channel, qotd_channel)]
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

/// Setting the channel id from the database for the server id in question
/// guild_id is from parsed within the command.
/// channel_id: String - Channel id to be set in the database
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

/// Pulls channel id from the database using the guild id.
/// Returns 0 if no result
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


async fn get_random_question() -> String {
    // Get random question from database and return it
    String::from("_")
}

async fn get_random_custom_question(guild_id: String) -> String {
    // Get random custom question from database and return it
    String::from("_")
}

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    println!("I'm help lmao");

    Ok(())
}

#[command]
async fn set_qotd_channel(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap(); // lazy solution, expecting the message to exist

    // If message is a valid message
    if msg.content.len() >= 19 {
        // Parsing channel id from the user message
        if let Some(cid) = parse_channel(&msg.content[19..]) {
            let channel_id = cid;

            // Calling function to set the the stuff to database
            set_channel_id(channel_id.to_string(), guild_id.to_string(), ctx).await?;
            msg.reply(ctx, "Channel set!").await?;
        }
        else {
            msg.reply(ctx, "Not a valid channel!").await?;
        }
    }
    // If message isn't long enough or something else broken in it
    else {
        msg.reply(ctx, "Not a valid channel!").await?;
    }


    Ok(())
}

#[command]
async fn qotd_channel(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap(); // lazy solution, expecting the message to exist

    let channel_id = get_channel_id(guild_id.to_string(), ctx).await;

    // Slightly convoluted. If the string returned is a 0, that means there was no result
    // This assumes channel id 0 does not exist on any server (safe assumption)
    // If the string returned isn't a 0, it's the id of the channel assigned
    // which is then formatted correctly for parse_channel.
    let channel_string;
    if channel_id != String::from("0") {
        channel_string = format!(
            "<#{}>",
            channel_id
        );
    }
    else {
        channel_string = String::from("None");
    }

    // Fails if string was 0 and there was no result. Please don't judge me for this solution.
    if let Some(_cid) = parse_channel(&channel_string) {
        msg.reply(ctx, format!("Qotd channel is set to {}", channel_string)).await?;
    }
    else {
        msg.reply(ctx ,"Channel not set!").await?;
    }

    Ok(())
}

#[command]
async fn qotd(ctx: &Context, msg: &Message) -> CommandResult {
    //posts qotd to channel indicated
    Ok(())
}

#[command]
async fn custom_qotd(ctx: &Context, msg: &Message) -> CommandResult {
    // chooses a qotd from customs for the server and posts

    Ok(())
}

#[command]
async fn submit_qotd(ctx: &Context, msg: &Message) -> CommandResult {
    // submits and saves qotd for the server
    // Might make submitting its own function outside the command

    Ok(())
}
