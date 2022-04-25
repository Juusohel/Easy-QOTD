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

use tokio_postgres::NoTls;

fn main() {
    println!("Hello, world!");
}
