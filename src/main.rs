use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html as HtmlResponse, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum_prometheus::PrometheusMetricLayer;
use scraper::{Html, Selector};

use serde::Serialize;

const CHARTS_URL: &str = "https://www.billboard.com/charts/japan-hot-100";
const PORT: u16 = 3001;

#[tokio::main]
async fn main() {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .layer(prometheus_layer);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", PORT))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
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

    let titles: Vec<_> = fragment
        .select(&selector)
        .map(|x| x.inner_html().trim().to_string())
        .collect();

    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse(".c-label.a-no-trucate").unwrap();

    let artists: Vec<_> = fragment
        .select(&selector)
        .map(|e| e.inner_html().trim().to_string())
        .collect();

    let mut songs = vec![];

    for (index, (artist, title)) in artists.into_iter().zip(titles.into_iter()).enumerate() {
        songs.push(Song {
            rank: (index + 1) as u8,
            song: title,
            artist,
        })
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
    songs: Vec<Song>,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => HtmlResponse(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}
