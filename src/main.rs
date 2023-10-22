use chrono::DateTime;
use serde::Deserialize;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use teloxide::prelude::*;

type DbConnectionType = Sqlite;
type DbPool = SqlitePool;
#[derive(Debug, Deserialize, sqlx::FromRow)]
struct User {
    id: i32,
    telegram_id: i32,
}

impl User {
    async fn insert_user_db(
        telegram_user_id: i32,
        pool: &Pool<DbConnectionType>,
    ) -> Result<(), ()> {
        let query_result =
            sqlx::query::<DbConnectionType>("INSERT INTO users (telegram_id) VALUES (?)")
                .bind(telegram_user_id)
                .execute(pool)
                .await;
        if query_result.is_err() {
            log::error!("Error inserting user in DB {:?}", query_result);
            return Err(());
        }
        return Ok(());
    }
    async fn select_all_user_db(pool: &Pool<DbConnectionType>) -> Result<Vec<User>, sqlx::Error> {
        return sqlx::query_as::<DbConnectionType, User>("SELECT * FROM users")
            .fetch_all(pool)
            .await;
    }
}

#[derive(Debug, Deserialize, sqlx::FromRow)]
struct Log {
    id: i32,
    status: bool,
    timestamp: String,
}

impl Log {
    async fn get_last_status_db(pool: &Pool<DbConnectionType>) -> Result<Log, sqlx::Error> {
        let db_last_state: Result<Log, sqlx::Error> =
            sqlx::query_as::<DbConnectionType, Log>("SELECT * FROM logs ORDER BY id DESC LIMIT 1")
                .fetch_one(pool)
                .await;
        if db_last_state.is_err() {
            log::error!("Error fetching last status in DB {:?}", db_last_state);
            return Err(db_last_state.unwrap_err());
        }
        let db_last_state = db_last_state.unwrap();
        return Ok(db_last_state);
    }
    async fn insert_status_db(
        current_real_status: bool,
        pool: &Pool<DbConnectionType>,
    ) -> Result<(), sqlx::Error> {
        return match sqlx::query::<DbConnectionType>(
            "INSERT INTO logs (status, timestamp) VALUES (?, ?)",
        )
        .bind(current_real_status)
        .bind(chrono::Local::now().to_rfc3339())
        .execute(pool)
        .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Error insert status in DB  {:?}", err);
                Err(err)
            }
        };
    }
}

async fn get_db() -> Pool<DbConnectionType> {
    let current_dir = std::env::current_dir().unwrap();
    let db_path = format!("{}/presencebot.db", current_dir.display());
    let pool = DbPool::connect(&db_path).await.unwrap();
    sqlx::query::<DbConnectionType>(
        r"
    CREATE TABLE IF NOT EXISTS logs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        status bool,
        timestamp TIMESTAMP
      );
      CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        telegram_id INTEGER,
        UNIQUE(telegram_id)
        );
    ",
    )
    .execute(&pool)
    .await
    .unwrap();
    return pool;
}

fn generate_response(status: bool, name: Option<String>) -> String {
    let current_hh_mm_ss = chrono::Local::now().format("%H:%M:%S").to_string();
    let current_dd_mm_yyyy = chrono::Local::now().format("%d/%m/%Y").to_string();

    if !status {
        //Closed
        return format!(
            "Il laboratorio è stato chiuso alle {} del {}",
            current_hh_mm_ss, current_dd_mm_yyyy
        );
    } else if status == true && name.is_some() {
        //Open with name
        return format!(
            "Il laboratorio è stato aperto alle {} del {} da {}",
            current_hh_mm_ss,
            current_dd_mm_yyyy,
            name.unwrap()
        );
    } else {
        //Open without name
        return format!(
            "Il laboratorio è stato aperto alle {} del {}",
            current_hh_mm_ss, current_dd_mm_yyyy
        );
    }
}

#[derive(Debug, Deserialize)]
struct LabState {
    id: u8,
    description: String,
}
#[derive(Debug, Deserialize)]
struct UserHistoryFetch {
    user: String,
    time: String,
}
async fn fetch_history() -> Option<String> {
    let get_lab_history_endpoint = std::env::var("GET_LAB_HISTORY_ENDPOINT").unwrap();
    let max_time_diff_history = std::env::var("HISTORY_INTERVAL").unwrap();
    let max_time_diff_history: i64 = max_time_diff_history.parse().unwrap();
    let current_state_lab_raw = reqwest::get(get_lab_history_endpoint).await;

    // handle the error
    if current_state_lab_raw.is_err() {
        log::error!("Error fetching lab history: {:?}", current_state_lab_raw);
        return None;
    }
    let val = current_state_lab_raw.unwrap();
    let parsed_resp: Vec<UserHistoryFetch> = val.json::<Vec<UserHistoryFetch>>().await.unwrap();

    let first_user = parsed_resp.first();
    if first_user.is_none() {
        return None;
    }
    let first_user = first_user.unwrap();
    let env_time_offset = std::env::var("CHRONO_TIME_OFFSET");
    let time_offset = env_time_offset.unwrap_or(String::from("+02:00")); //Z = UTF, +02:00 italy,ecc
    let time = format!("{}{}", first_user.time, time_offset);
    let time_parsed = DateTime::parse_from_rfc3339(&time);
    if time_parsed.is_err() {
        log::error!("Error parsing time: {:?} , {}", time_parsed, time);
        return None;
    }
    let time_parsed = time_parsed.unwrap();
    let current_time = chrono::Local::now();
    let time_diff = current_time
        .signed_duration_since(time_parsed)
        .num_seconds();
    if time_diff < max_time_diff_history {
        return Some(first_user.user.clone());
    }
    return None;
}

async fn fetching_state_loop(pool: &Pool<DbConnectionType>) {
    let get_lab_state_endpoint = std::env::var("GET_LAB_STATE_ENDPOINT").unwrap();

    let current_state_lab_raw = reqwest::get(get_lab_state_endpoint).await;
    let mut current_state_lab_id: bool = false;
    if let Ok(data) = current_state_lab_raw {
        let current_state_lab: LabState = data.json().await.unwrap();
        log::info!("Current state 1: {:?}", current_state_lab);
        current_state_lab_id = current_state_lab.id == 1; //Casting int to bool
    }
    log::info!("Current state 2: {:?}", current_state_lab_id);
    let db_last_state = Log::get_last_status_db(pool).await;

    match db_last_state {
        Ok(data) => {
            let db_last_status = data.status;
            if db_last_status == current_state_lab_id {
                log::info!("Current state: equal to last state");
                return;
            }
            Log::insert_status_db(current_state_lab_id, pool)
                .await
                .unwrap();

            let history_user_name = fetch_history().await;
            let message: String = generate_response(current_state_lab_id, history_user_name);

            let users = User::select_all_user_db(pool).await.unwrap();
            let bot = Bot::from_env();

            for user in users.iter() {
                log::info!("Sending message to user: {:?}", user);

                let user_id = user.telegram_id as i64;
                match bot.send_message(ChatId(user_id), &message).send().await {
                    Ok(_) => {
                        log::info!("Message sent to user: {:?}", user);
                    }
                    Err(err) => {
                        log::error!("Error sending message to user: {:?} , {:?}", user, err);
                    }
                }
            }
        }
        Err(_) => {
            log::info!("Current state: no last state, inserting genesis status in DB");

            Log::insert_status_db(current_state_lab_id, pool)
                .await
                .unwrap();
        }
    };
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    let polling_interval: String = std::env::var("POLLING_INTERVAL").unwrap_or("60".to_string());
    let polling_interval: u64 = polling_interval.parse().unwrap_or(60);

    let pool = get_db().await;
    let pool_loop = pool.clone();
    let pool_telegram = pool.clone();
    //Loop fetch state
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(polling_interval));
        loop {
            interval.tick().await;
            log::info!("Fetching state LAB...");
            fetching_state_loop(&pool_loop).await;
        }
    });
    //Loop telegram bot
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        loop_telegram(bot, msg, pool_telegram.clone())
    })
    .await;
}

async fn loop_telegram(bot: Bot, msg: Message, db: Pool<DbConnectionType>) -> ResponseResult<()> {
    let user_add_try = User::insert_user_db(msg.chat.id.0 as i32, &db).await;

    let db_last_state = Log::get_last_status_db(&db).await;
    let mut message_status: Option<String> = None;
    if let Ok(db_last_state) = db_last_state {
        if db_last_state.status {
            message_status = Some(String::from("Il laboratorio è attualmente aperto"));
        } else {
            message_status = Some(String::from("Il laboratorio è attualmente chiuso"));
        }
    }

    let messaggio_benvenuto = match user_add_try {
        Ok(()) =>  "Ciao, sono il bot HLCS 🦀. Ti avviserò quando il laboratorio sarà aperto o chiuso",
        Err(()) =>  "Ciao, sono il bot HLCS 🦀. Ti avviserò quando il laboratorio sarà aperto o chiuso, ti avverto che eri già iscritto"
    };
    bot.send_message(msg.chat.id, messaggio_benvenuto).await?;

    if message_status.is_some() {
        bot.send_message(msg.chat.id, message_status.unwrap())
            .await?;
    }
    Ok(())
}
