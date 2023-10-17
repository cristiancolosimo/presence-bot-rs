Presence Telegram Bot (RS 🦀 version)
===
A Bot telegram for checking the status of door of the hacklab


For use it copy the file .env.example to .env (or set the env vars in other way in docker-compose.yml)

and set the .env vars:

- TELOXIDE_TOKEN , API tokey generated by the @BotFather

- POLLING_INTERVAL default 60 seconds

- HISTORY_INTERVAL default 125 seconds

- GET_LAB_STATE_ENDPOINT, the endpoint for get the state of the lab

- GET_LAB_HISTORY_ENDPOINT, the endpoint for get the history of opens of the lab

- RUST_LOG=info, the log level of the bot (trace, debug, info, warn, error)

- CHRONO_TIME_OFFSET="+02:00", the offset of the time zone of the lab, default +02:00 (Italy, Rome), for other time zone see https://docs.rs/chrono/latest/chrono/offset/struct.FixedOffset.html
- TZ=Europe/Rome , the time zone of the lab, default Europe/Rome, for other time zone see https://en.wikipedia.org/wiki/List_of_tz_database_time_zones used by alpine linux

and for start it run the command docker-compose up -d 
