CREATE TABLE channels (
guild_id varchar PRIMARY KEY,
channel_id varchar NOT NULL
);

CREATE TABLE questions (
    question_id serial PRIMARY KEY,
    question_string varchar NOT NULL,
    in_use bool NOT NULL
);

CREATE TABLE custom_questions (
    question_id serial PRIMARY KEY,
    guild_id varchar NOT NULL,
    question_string varchar NOT NULL
);

CREATE TABLE ping_roles (
    guild_id varchar PRIMARY KEY,
    ping_role varchar NOT NULL
);

CREATE TABLE polls (
    poll_id serial PRIMARY KEY,
    poll_string varchar[] NOT NULL,
    in_use bool NOT NULL
);

CREATE TABLE custom_polls (
    poll_id serial PRIMARY KEY,
    guild_id varchar NOT NULL,
    poll_string varchar[] NOT NULL
);
