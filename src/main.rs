mod config;
mod providers;

use crate::config::{RequestMode, load_config};

use crate::providers::{CustomSessionProvider, MojangSessionProvider, SessionProvider};
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use futures::future;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

struct ServerData {
    session_provider: Arc<MojangSessionProvider>,
    auth_providers: Vec<Arc<dyn SessionProvider>>,
    request_mode: RequestMode,
}

#[derive(Deserialize)]
struct HasJoinedQuery {
    username: String,
    serverId: String,
    providers: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct ProfileQuery {
    unsigned: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ProfileProperty {
    name: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    properties: Vec<ProfileProperty>,
}

#[get("/session/minecraft/profile/{uuid}")]
async fn get_profile(
    path: web::Path<String>,
    query: web::Query<ProfileQuery>,
    server: web::Data<ServerData>,
) -> impl Responder {
    let uuid = path.into_inner();
    if uuid.len() != 32 || uuid.len() != 36 {
        println!("✘ Invalid UUID {}", uuid);
        return HttpResponse::Unauthorized().finish();
    }

    let res = server
        .session_provider
        .profile(&uuid, query.unsigned.unwrap_or(false))
        .await;

    match res {
        Some(profile) => HttpResponse::Ok().json(profile),
        None => HttpResponse::NotFound().finish(),
    }
}

#[get("/session/minecraft/hasJoined")]
async fn has_joined(
    query: web::Query<HasJoinedQuery>,
    server: web::Data<ServerData>,
) -> impl Responder {
    let username = &query.username;
    let server_id = &query.serverId;

    if username.is_empty() || username.len() > 16 {
        return HttpResponse::BadRequest().body("invalid username");
    }

    if server_id.is_empty() || server_id.len() > 256 {
        return HttpResponse::BadRequest().body("invalid serverId");
    }

    let timer = Instant::now();

    let mut auth_providers: Vec<Arc<dyn SessionProvider>> = server.auth_providers.clone();
    match &query.providers {
        Some(providers) => {
            if !providers.is_empty() {
                auth_providers.retain(|provider| {
                    providers
                        .iter()
                        .any(|f| f.eq_ignore_ascii_case(&provider.name().to_string()))
                });
            }
        }
        None => {}
    }

    let result: Option<(String, ProfileResponse)> = match server.request_mode {
        RequestMode::Parallel => {
            let futures = auth_providers.iter().map(|provider| {
                let username = username.clone();
                let server_id = server_id.clone();
                let provider = Arc::clone(provider);

                Box::pin(async move {
                    match provider.has_joined(&username, &server_id).await {
                        Some(profile) => Ok((provider.name().to_string(), profile)),
                        None => Err(()),
                    }
                })
            });

            future::select_ok(futures).await.map(|(r, _)| r).ok()
        }
        RequestMode::Sequential => {
            let mut valid_provider = None;
            for provider in auth_providers.iter() {
                if let Some(profile) = provider.has_joined(username, server_id).await {
                    valid_provider = Some((provider.name().to_string(), profile));
                    break;
                }
            }
            valid_provider
        }
    };

    match result {
        Some((provider, profile)) => {
            println!(
                "✔ Authed {} using {} session provider in {}ms",
                username,
                provider,
                timer.elapsed().as_millis()
            );
            HttpResponse::Ok().json(profile)
        }
        None => {
            println!("✘ Failed to authenticate {}", username);
            HttpResponse::Unauthorized().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config =
        load_config(&std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string()));

    let mojang_provider: Arc<dyn SessionProvider> = Arc::new(MojangSessionProvider);

    let mut provider_map: HashMap<String, Arc<dyn SessionProvider>> = HashMap::new();
    provider_map.insert(mojang_provider.name().to_lowercase(), mojang_provider);

    for provider in config
        .custom_providers
        .into_iter()
        .map(|c| Arc::new(CustomSessionProvider::new(c.name, c.url)) as Arc<dyn SessionProvider>)
    {
        provider_map.insert(provider.name().to_lowercase(), provider);
    }

    let auth_providers: Vec<Arc<dyn SessionProvider>> = if config.request_order.is_empty() {
        provider_map.into_values().collect()
    } else {
        config
            .request_order
            .iter()
            .filter_map(|provider_name| {
                let provider = provider_map.remove(&provider_name.to_lowercase());
                if provider.is_none() {
                    eprintln!("Unknown provider in order: {}", provider_name);
                }
                provider
            })
            .collect()
    };

    let server_data = web::Data::new(ServerData {
        session_provider: Arc::new(MojangSessionProvider),
        auth_providers,
        request_mode: config.request_mode,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(server_data.clone())
            .service(has_joined)
            .service(get_profile)
    })
    .bind(("0.0.0.0", 20005))?
    .run()
    .await
}
