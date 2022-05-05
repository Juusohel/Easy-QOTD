# Easy Question of the Day
Flexible discord bot for posting questions of the day with possibility for custom questions and more!

## Features
With an attached database of questions, the bot will post a random question of the day on demand. 
Server administrators are able to set specific channels and roles to ping when sending a question.
There is also support for custom questions!

WIP Features
- Qotd Scheduling
- Reaction polls



## Configuration
###### Environment Variables
- `DISCORD_TOKEN` - Discord token for the bot
- `DB_CONNECTION` - Connection string to the database used by the bot
    - `host=<> dbname=<> user=<> password=<>`- 


###### Other Settings
- Permissions: Administrators or users with the role **qotd_admin**
- Command Prefix - `q!`

## Requirements and dependencies
#### Rust toolchain
- Easiest way to install the rust toolchain is with rustup
    - https://www.rust-lang.org/tools/install

#### serenity
- Rust library for the Discord API
    - https://github.com/serenity-rs/serenity

#### tokio
- Rust runtime for asynchronous applications
    - https://github.com/tokio-rs/tokio

#### tokio-postgres
- PostgreSQL support for rust
    - https://github.com/sfackler/rust-postgres

## Acknowledgements
Thanks to DioritePoodle for help with writing default questions