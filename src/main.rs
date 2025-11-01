use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_cors::Cors;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use log::info;

mod models;
mod handlers;
mod services;
mod utils;

use handlers::a2a::{handle_a2a_request, AppState};
use services::github::GitHubClient;
use services::scanner::SecretScanner;
use services::gemini::GeminiClient;
use services::state::StateManager;

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "secret-detector"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let github_token = env::var("GITHUB_TOKEN").ok();
    let gemini_api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let gemini_model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());
    let max_scan_commits = env::var("MAX_SCAN_COMMITS")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<usize>()
        .unwrap_or(100);
    let scan_state_file = env::var("SCAN_STATE_FILE").unwrap_or_else(|_| "scan_states.json".to_string());

    let github_client = GitHubClient::new(github_token).expect("Failed to create GitHub client");
    let scanner = SecretScanner::new();
    let gemini_client = GeminiClient::new(gemini_api_key, gemini_model);
    let state_manager = Arc::new(StateManager::new(&scan_state_file).expect("Failed to create state manager"));

    let app_state = web::Data::new(AppState {
        github_client,
        scanner,
        gemini_client,
        state_manager,
        max_scan_commits,
    });

    let bind_addr = format!("{}:{}", host, port);
    info!("Starting server on {}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            .route("/health", web::get().to(health_check))
            .route("/a2a/agent/githubScanner", web::post().to(handle_a2a_request))
    })
    .bind(&bind_addr)?
    .run()
    .await
}