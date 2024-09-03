use axum::{Router, routing::{get, post}};
use axum::extract::Json;
use axum::response::{Response, IntoResponse};
use axum::http::StatusCode; 
use serde_json::json;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::{channel, Sender};
use regex::Regex;
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};
use std::net::SocketAddr;
use indicatif::ProgressBar;

#[derive(Serialize, Deserialize, Debug)]
struct Artist {
    id: u64,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DownloadItem {
    url: String,
    id: String,
    post: String,
    name: String,
    ext: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DownloadRequest {
    artist: String,
    data: Vec<DownloadItem>,
}

const JS: &str = include_str!("script.js");

async fn handle_gadget() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::from(JS))
        .unwrap()
}

async fn handle_user(Json(artist): Json<Artist>) -> impl IntoResponse {
    println!("{:#?}", artist);
    (StatusCode::OK, Json(json!({"status": "success"})))
}

async fn handle_download(Json(download_request): Json<DownloadRequest>, tx: Sender<DownloadRequest>) -> impl IntoResponse {
    match tx.send(download_request).await {
        Ok(_) => (StatusCode::OK, Json(json!({"status": "success"}))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "error"}))),
    }
}

async fn worker(mut rx: tokio::sync::mpsc::Receiver<DownloadRequest>) {
    while let Some(download_request) = rx.recv().await {
        let artist = download_request.artist.clone();
        
        let progress_bar = ProgressBar::new(download_request.data.len() as u64);
        
        let mut download_tasks = vec![];

        for item in download_request.data {
            let artist = artist.clone();
            let url = item.url.clone();
            let filename = format!("{}-{}-{}.{}", item.post, item.id, sanitize(&item.name), item.ext);
            let filepath = format!("patreon/{}/{}", artist, filename);
            let progress_bar = progress_bar.clone();

            let handle = tokio::spawn(async move {
                fetch_and_save(url, filepath).await;
                progress_bar.inc(1);
            });

            download_tasks.push(handle);
        }
        
        for task in download_tasks {
            match task.await {
                Ok(_) => {},
                Err(e) => eprintln!("Task failed: {:?}", e),
            }
        }

        progress_bar.finish_with_message("Download complete");
    }
}

async fn fetch_and_save(url: String, path: String) {
    if std::path::Path::new(&path).exists() {
        println!("Already exists");
        return;
    }

    let res = match reqwest::get(&url).await {
        Ok(response) => response,
        Err(_) => {
            println!("Error downloading {}", url);
            return;
        }
    };

    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(_) => {
            println!("Error reading bytes from {}", url);
            return;
        }
    };

    if let Some(parent) = std::path::Path::new(&path).parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            println!("Error creating directories: {}", e);
            return;
        }
    }

    if let Err(e) = tokio::fs::write(&path, &bytes).await {
        println!("Error writing to file: {}", e);
    }
}

fn sanitize(filename: &str) -> String {
    let re = Regex::new(r"[^\w\-.]").unwrap();
    re.replace_all(filename, "_").to_string().chars().take(255).collect()
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let (tx, rx) = channel::<DownloadRequest>(100);

    let app = Router::new()
        .route("/gadget", get(handle_gadget))
        .route("/user", post(handle_user))
        .route("/download", post({
            let tx = tx.clone();
            move |Json(download_request)| handle_download(Json(download_request), tx.clone())
        }))
        .layer(ServiceBuilder::new().layer(cors));

    tokio::spawn(worker(rx));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Server running at http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}