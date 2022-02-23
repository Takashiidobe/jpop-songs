use axum::{
    routing::{get},
    Json, Router,
    http::StatusCode,
    response,
    response::{IntoResponse, Response},
    body::{self, Full}
};
use askama::Template;
use std::net::SocketAddr;
use scraper::{Html, Selector};

use serde::{Serialize};

const CHARTS_URL: &str = "https://www.billboard.com/charts/japan-hot-100";
const PORT: u16 = 3001;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/api", get(api));

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> impl IntoResponse {
    let songs = get_songs().await;
    let template = Songs { songs };
    (StatusCode::OK, HtmlTemplate(template))
}

async fn api() -> impl IntoResponse {
    let songs = get_songs().await;
    (StatusCode::OK, Json(songs))
}

async fn get_songs() -> Vec<Song> {
        let html = reqwest::get(CHARTS_URL)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse(".c-title.a-no-trucate").unwrap();

    let mut titles = vec![];

    for element in fragment.select(&selector) {
        titles.push(element.inner_html());
    }

    let titles: Vec<_> = titles.into_iter().map(|x| x.trim().to_string()).collect();

    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse(".c-label.a-no-trucate").unwrap();

    let mut artists = vec![];

    for element in fragment.select(&selector) {
        artists.push(element.inner_html());
    }

    let artists: Vec<String> = artists.into_iter().map(|x| x.trim().to_string()).collect();

    let mut songs = vec![];

    for (index, (artist, title)) in artists.into_iter().zip(titles.into_iter()).enumerate() {
        songs.push(
            Song {
                rank: (index + 1) as u8,
                song: title,
                artist,
            }
        )
    }

    songs
}

#[derive(Debug, Serialize)]
struct Song {
    rank: u8,
    artist: String,
    song: String,
}

#[derive(Template)]
#[template(path = "songs.html")]
struct Songs {
    songs: Vec<Song>
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    name: String,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => response::Html(html).into_response(),
            Err(err) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(body::boxed(Full::from(format!(
                    "Failed to render template. Error: {}",
                    err
                ))))
                .unwrap(),
        }
    }
}
