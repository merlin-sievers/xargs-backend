use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use clap::Parser;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: PathBuf,
}

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    highscore_dir: PathBuf,
    ip: String,
    port: u16,
    games: Vec<GameConfig>,
}

#[derive(Deserialize)]
struct GameConfig {
    name: String,
    max_highscores: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            highscore_dir: "./scores".into(),
            ip: "127.0.0.1".into(),
            port: 8484,
            games: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[serde(untagged)]
enum ScoreValue {
    Int(i64),
}

#[derive(Deserialize, Serialize)]
struct Highscore {
    name: String,
    score: ScoreValue,
}

struct AppState {
    config: Config,
    allowed_games: HashSet<String>,
    cache: Mutex<HashMap<String, Vec<Highscore>>>,
}

#[derive(Deserialize)]
struct ScoreSubmission {
    name: String,
    score: ScoreValue,
    i_realize_that_cheating_is_not_fun_and_ruins_fun_projects: String,
}

#[post("/submit/{game}")]
async fn submit_score(
    data: web::Data<AppState>,
    path: web::Path<String>,
    payload: web::Json<ScoreSubmission>,
) -> impl Responder {
    let game = path.into_inner();
    let submission = payload.into_inner();

    if !data.allowed_games.contains(&game) {
        error!("Requested unknown game {game}");
        return HttpResponse::BadRequest().body("unknown game");
    }
    if submission.i_realize_that_cheating_is_not_fun_and_ruins_fun_projects != "yes" {
        return HttpResponse::Forbidden().finish();
    }

    let game_config = data.config.games.iter().filter(|gc| gc.name == game).next().unwrap();

    let max_highscores = game_config.max_highscores;
    let score_to_insert = submission.score;
    let name = submission.name.clone();

    if name.len() != 4 {
        error!(
            "Someone tried to submit a highscore under the name {name} which does not have exactly 4 characters."
        );
        return HttpResponse::BadRequest().body("name has to be exactly 4 characters");
    }

    if !name.chars().all(|c| c.is_ascii_lowercase()) {
        error!("Someone tried to submit a highscore under a name that is not all lowercase ascii.");
        return HttpResponse::BadRequest()
            .body("name can only be lowercase ascii (a-z) characters");
    }

    let mut cache = data.cache.lock().unwrap();

    match cache.get_mut(&game) {
        Some(scores) => {
            let insert_position = scores
                .iter()
                .position(|x| x.score >= score_to_insert)
                .unwrap_or(scores.len());
            scores.insert(
                insert_position,
                Highscore {
                    name,
                    score: score_to_insert,
                },
            );
            while scores.len() > max_highscores {
                scores.remove(max_highscores);
            }
            match serialize_cache(&cache, &data.config) {
                Ok(()) => HttpResponse::Ok().finish(),
                Err(_) => HttpResponse::InternalServerError().body("Your submission could not be persisted. It might be lost in the future :( Please contact me about this!")
            }

        }
        None => {
            error!("Did not find game {game} in the cache even though it is an allowed game.");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn serialize_cache(cache: &HashMap<String, Vec<Highscore>>, config: &Config) -> std::io::Result<()> {
    let highscore_dir = &config.highscore_dir;
    if let Err(err) = fs::create_dir_all(highscore_dir) {
        error!(
            "Failed to create directory {}: {}",
            highscore_dir.display(),
            err
        );
        return Err(err);
    }

    for (game, scores) in cache.iter() {
        let file_path = highscore_dir.join(format!("{}.json", game));
        let json = match serde_json::to_string_pretty(scores) {
            Ok(j) => j,
            Err(err) => {
                error!(
                    "Failed to serialize highscores {}: {}",
                    file_path.display(),
                    err
                );
                return Err(err.into());
            }
        };

        if json.is_empty() {
            continue;
        }

        if let Err(err) = fs::write(&file_path, &json) {
            error!("Failed to write {}: {}", file_path.display(), err);
            return Err(err);
        }
    }

    Ok(())
}

#[get("/highscores/{game}")]
async fn highscores(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let game = path.into_inner();

    if !data.allowed_games.contains(&game) {
        error!("Requested unknown game {game}");
        return HttpResponse::BadRequest().body("unknown game");
    }

    let mut cache = data.cache.lock().unwrap();

    if !cache.contains_key(&game) {
        let filepath = format!("{game}.json");
        let file = data.config.highscore_dir.join(filepath);

        let text = match fs::read_to_string(&file) {
            Ok(t) => t,
            Err(_) => {
                error!("Failed to read file: {}", file.display());
                return HttpResponse::NotFound().finish();
            }
        };

        let parsed: Vec<Highscore> = match serde_json::from_str(&text) {
            Ok(hs) => hs,
            Err(_) => {
                error!("Failed to parse highscore entries for game {game}");
                return HttpResponse::InternalServerError().finish();
            }
        };

        cache.insert(game.clone(), parsed);
    }

    let scores = cache.get(&game);
    match scores {
        Some(scores) => {
            let response: Vec<(String, String)> = scores
                .iter()
                .map(|s| (s.name.clone(), serde_json::to_string(&s.score).unwrap()))
                .collect();

            HttpResponse::Ok().json(response)
        }
        None => {
            error!("Could not get cached highscores for game {game}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config_text = fs::read_to_string(cli.config).expect("config read failed");
    let config: Config = toml::from_str(&config_text).expect("config parse failed");
    let allowed_games = config.games.iter().map(|gc| gc.name.clone()).collect();

    let connect_pair = (config.ip.clone(), config.port);

    let state = web::Data::new(AppState {
        config,
        allowed_games,
        cache: Mutex::new(HashMap::new()),
    });

    info!("Listening on {}:{}", &connect_pair.0, &connect_pair.1);
    HttpServer::new(move || App::new().app_data(state.clone()).service(highscores))
        .bind(connect_pair)?
        .run()
        .await
}
