use std::sync::Arc;
use std::{env, fs, io};
use std::fmt::format;


use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};

use serenity::utils::{Color, parse_channel, parse_message_url, parse_role};
use serenity::{async_trait, Error, model::{channel::Message, gateway::Ready}, prelude::*};
use serenity::model::id::{ChannelId, GuildId};

use tokio_postgres::{NoTls, Row, RowStream};


// Container for psql client
struct DataClient {
    _tokio_postgres: tokio_postgres::Client,
}

impl TypeMapKey for DataClient {
    type Value = Arc<tokio_postgres::Client>;
}

// General framework for commands
#[group]
#[commands(help, set_qotd_channel, qotd_channel, qotd, custom_qotd, submit_qotd, delete_question, customs, pingrole)]
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

/// Pulls channel id formatted for parse_channel() from the database using the guild id.
/// Returns "0" if no result
async fn get_channel_id(guild_id: String, ctx: &Context) -> String {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let mut channel_id: String;
    let rows = client
        .query(
            "SELECT channel_id FROM channels WHERE guild_id = $1",
            &[&guild_id]
        )
        .await
        .expect("Error querying database");
    let channel_string;
    if rows.len() > 0 {
        channel_id = rows[0].get(0);
        channel_string = format!(
            "<#{}>",
            channel_id
        );
    }
    else {
        channel_string = String::from("0");
    }
    channel_string
}

/// Gets a random question from the database and returns it as a string
async fn get_random_question(ctx: &Context) -> String {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();


    // Getting a random entry from the database by querying the database with random order and displaying one.
    // NOTE: This is rather inefficient because the function in psql is slow, and not exactly efficient
    // Future implementations might make this a bit faster but while there isn't thousands of question this will work fine
    // Using a random number generator with the multi-threading was kinda annoying and since there's less than 1000 entries, this should be fine, for now.
    let rows = client
        .query(
            "SELECT question_string FROM questions WHERE in_use = $1 ORDER BY random() LIMIT 1",
            &[&true]
        )
        .await
        .expect("Error querying database");

    let question_string= rows[0].get(0);

    question_string



}

/// Adds a custom question to the database with the associated guild_id
async fn add_custom_question(guild_id: String, question: String, ctx: &Context) -> Result<u64, tokio_postgres::Error> {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let insert = client
        .execute(
            "INSERT INTO custom_questions (guild_id, question_string) VALUES ($1, $2)",
            &[&guild_id, &question]
        )
        .await;

    insert
}

/// Deletes a specified question from the database.
/// Using the guild_id provided, the function checks ownership of the question matches the ID.
/// If match, the question is deleted.
/// Returns 1 on successful deletion
/// Returns 0 if deletion failed.
async fn delete_custom_question(guild_id: String, question_id: i32, ctx: &Context) -> i32 {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    // Checking if a question with the guild_id of the requesting server exists, if it exists, delete the question.
    // This prevents from other servers deleting each others questions.
    let rows = client
        .query(
            "SELECT * FROM custom_questions WHERE guild_id = $1 AND question_id = $2",
            &[&guild_id, &question_id]
        )
        .await
        .expect("Select Failed");
    if rows.len() > 0 {
        let delete = client.execute(
            "DELETE FROM custom_questions WHERE question_id = $1",
            &[&question_id]
        )
            .await
            .expect("Delete failed");

        1
    }
    else {
        0
    }

}

/// Gets all the questions submitted by the guild_id and returns vector of rows
async fn get_list_custom_questions(guild_id: String, ctx: &Context) -> Vec<Row> {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let rows = client
        .query(
            "SELECT * FROM custom_questions WHERE guild_id = $1",
            &[&guild_id]
        )
        .await
        .expect("Error querying database");

    rows
}

/// Queries the database for a custom question
async fn get_random_custom_question(guild_id: String, ctx: &Context) -> String {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let rows = client
        .query(
            "SELECT question_string FROM custom_questions WHERE guild_id = $1 ORDER BY random() LIMIT 1",
            &[&guild_id]
        )
        .await
        .expect("Error querying database");

    if rows.len() > 0 {
        let question_string= rows[0].get(0);

        question_string
    }
    else {
        let question_string = String::from("No custom questions found!");
        question_string
    }


}

/// Gets a specific custom question from the database based on id
async fn get_specific_custom(guild_id: String, question_id: i32, ctx: &Context) -> String {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let rows = client
        .query(
            "SELECT question_string FROM custom_questions WHERE guild_id = $1 AND question_id = $2",
            &[&guild_id, &question_id]
        )
        .await
        .expect("Error querying database");

    if rows.len() > 0 {
        rows[0].get(0)
    }
    else {
        String::from("Question does not exist!")
    }
}

/// Saves a role id to be used to ping into the database.
/// guild_id is the id of the server the command is called from.
/// 0 is used for no ping
/// 1 is used for EVERYONE
/// submitted id is used for specific role
async fn set_ping_role(guild_id: String, ping_role: String, ctx: &Context) -> Result<u64, tokio_postgres::Error> {
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let upsert = client
        .execute(
            "INSERT INTO ping_roles (guild_id, ping_role)
            VALUES ($1, $2)
            ON CONFLICT (guild_id)
            DO
            UPDATE SET ping_role = EXCLUDED.ping_role",
            &[&guild_id, &ping_role]
        )
        .await;

    upsert
}

/// Gets the role id to be used for pinging based on the guild_id
///  0 is used for no ping
/// 1 is used for EVERYONE
/// submitted id is used for specific role
async fn get_ping_role(guild_id: String, ctx: &Context) -> String{
    // Pulling in psql client
    let read = ctx.data.read().await;
    let client = read
        .get::<DataClient>()
        .expect("PSQL Client error")
        .clone();

    let rows = client
        .query(
            "SELECT ping_role FROM ping_roles WHERE guild_id = $1",
            &[&guild_id]
        )
        .await
        .expect("Error querying database");

    // Return the ping role as string
    if rows.len() > 0 {
        rows[0].get(0)
    }
    else {
        //Return 0 if there's no ping role assigned
        String::from("0")
    }

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
    // which is then used for parse_channel.

    // Fails if string was 0 and there was no result. Please don't judge me for this solution.
    if let Some(_cid) = parse_channel(&channel_id) {
        msg.reply(ctx, format!("Qotd channel is set to {}", channel_id)).await?;
    }
    else {
        msg.reply(ctx ,"Channel not set!").await?;
    }

    Ok(())
}

#[command]
async fn qotd(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let question = get_random_question(ctx).await;
    let channel_id = get_channel_id(guild_id.to_string(), ctx).await;

    if let Some(cid) = parse_channel(&channel_id) {
        // Sending message to the channel assigned to the server
        let channel = ChannelId(cid);
        channel.send_message(ctx, |message| {
            message
                .content(format!(
                    "{}", question
                ))
        })
            .await?;
    }
    else {
        msg.reply(ctx ,"Channel not set!").await?;
    }

    Ok(())


}

#[command]
async fn custom_qotd(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let custom_question;
    let channel_id = get_channel_id(guild_id.to_string(),ctx).await;


    if msg.content.len() >= 14 {
        if let Ok(id_to_use) =  &msg.content[14..].parse::<i32>() {
            let id_to_use = *id_to_use;
            custom_question = get_specific_custom(guild_id.to_string(), id_to_use, ctx).await;
        }
        else {
            msg.reply(ctx, "Not a valid question ID").await?;
            return Ok(());
        }

    }
    else {
        custom_question = get_random_custom_question(guild_id.to_string(), ctx).await;
    }

    if let Some(channel) = parse_channel(&channel_id) {
        // Sending message to the channel assigned to the server
        let channel = ChannelId(channel);
        channel.send_message(ctx, |message| {
            message
                .content(format!(
                    "{}", custom_question
                ))
        })
            .await?;
    }
    else {
        msg.reply(ctx ,"Channel not set!").await?;
    }

    Ok(())
}

#[command]
async fn submit_qotd(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let user_submission;

    // Could add regex for bad words etc here.
    // If message is valid
    if msg.content.len() >= 14 {
        user_submission = &msg.content[14..];
        if let Ok(_s) = add_custom_question(guild_id.to_string(), user_submission.to_string(), ctx).await {
            msg.reply(ctx,"Question Submitted").await?;
        }
        else {
            msg.reply(ctx, "Something went wrong!").await?;
        }
    }
    else {
        msg.reply(ctx,"Question not accepted").await?;
    }

    Ok(())
}

#[command]
async fn delete_question(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    if msg.content.len() >= 18 {
        // Parsing id from the message
        if let Ok(id_to_delete) =  &msg.content[18..].parse::<i32>() {
            let id_to_delete = id_to_delete;
            let test = delete_custom_question(guild_id.to_string(), *id_to_delete, ctx).await;
            if test == 1 {
                msg.reply(ctx,"Question deleted!").await?;
            }
            else {
                msg.reply(ctx, "Question not found!").await?;
            }
        }
            // If id not able to be parsed
        else {
            msg.reply(ctx, "Please enter a valid ID!").await?;
        }
    }
    else {
        // Getting all questions
        let question_list = get_list_custom_questions(guild_id.to_string(), ctx).await;

        // If there are custom questions saved
        if question_list.len() > 0 {
            // Formatting vector for printing
            let length = question_list.len();

            let mut pretty_list = "ID - Question\n".to_string();
            // Putting the questions onto the list
            for i in 0..length {
                let qid: i32 = question_list[i].get(0);
                let string: String = question_list[i].get(2);
                pretty_list = format!("{}{} - {} \n", pretty_list, qid, string)
            }
            // Listing questions in message
            msg.channel_id
                .send_message(ctx, |m| {
                    m
                        .content(format!("<@{}> Please specify the ID of question",
                                         msg.author.id
                        ))
                        .embed(|embed| {
                            embed
                                .title("Questions")
                                .description(pretty_list)
                                .color(Color::DARK_BLUE)
                        })
                })
                .await?;
        }
        else {
            msg.reply(ctx,"No custom questions found!").await?;
        }

    }

    Ok(())
}

#[command]
async fn customs(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    // Getting all questions
    let question_list = get_list_custom_questions(guild_id.to_string(), ctx).await;

    // If there are custom questions saved
    if question_list.len() > 0 {
        // Formatting vector for printing
        let length = question_list.len();

        let mut pretty_list = "ID - Question\n".to_string();
        // Putting the questions onto the list
        for i in 0..length {
            let qid: i32 = question_list[i].get(0);
            let string: String = question_list[i].get(2);
            pretty_list = format!("{}{} - {} \n", pretty_list, qid, string)
        }
        // Listing questions in message
        msg.channel_id
            .send_message(ctx, |m| {
                m
                    .content(format!("<@{}> Here's a list of all saved custom questions",
                                     msg.author.id
                    ))
                    .embed(|embed| {
                        embed
                            .title("Questions")
                            .description(pretty_list)
                            .color(Color::RED)
                    })
            })
            .await?;
    }
    else {
        msg.reply(ctx,"No custom questions found!").await?;
    }


    Ok(())
}

/// Command to set ping role
#[command]
async fn pingrole(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let mut current_role = get_ping_role(guild_id.to_string(),ctx).await;

    // Checking if there's parameters in the command
    if msg.content.len() >= 11 {
        let parameter = &msg.content[11..];

        // If role parameter is one of the preset options
        if parameter == "1" || parameter == "0" {
            if let Ok(_) = set_ping_role(guild_id.to_string(), String::from(parameter), ctx).await {
                msg.reply(ctx, "Ping role updated!").await?;
            }
            else {
                msg.reply(ctx, "Something went wrong!").await?;
            }
        }
        // Else check whether the role is valid, and submit it if it is
        else {
            // If role is a valid role, submit it to the database
            if let Some(role) = parse_role(parameter) {
                if let Ok(_) = set_ping_role(guild_id.to_string(), role.to_string(), ctx).await {
                    msg.reply(ctx, "Ping role updated!").await?;
                }
                else {
                    msg.reply(ctx, "Something went wrong!").await?;
                }
            }
            else {
                msg.reply(ctx, "Not a valid role!").await?;
            }
        }
    }
    // If no parameters, send default help message
    else {
        // Formatting current role to taggable form if it's not 0 or 1
        if (current_role != String::from("1")) && (current_role != String::from("0")) {
            // No need to check if the role is a valid role, validity is checked on submission to the database.
            current_role = format!("<@&{}>", current_role);
        }

        msg.channel_id.send_message(ctx, |m| {
            m
                .content(format!(
                    "<@{}> Use this command to set the role to be pinged when posting a qotd \n \
                    Current setting is {}",
                    msg.author.id,
                    current_role
                ))
                .embed(|embed| {
                    embed
                        .title("Parameters")
                        .description("<role> - Specific role \n 1 - Everyone \n 0 - Off (default)")
                })
        })
            .await?;
    }

    Ok(())
}

// TODO: Ability to add role to ping (similar to guild_id checks)
// TODO: Message looks ok
// TODO: Commands better looking (change the message variable thing)
// TODO: Timer and permissions
