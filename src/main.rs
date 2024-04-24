use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};

use std::{sync::Arc};

use axum::Form;
use axum::extract::State;
use axum::routing::post;

use std::sync::Mutex;


use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct AppState {
    todos: Mutex<Vec<String>>,
}
#[tokio::main] 
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "with_axum_htmx_askama=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("initializing router...");

    let app_state = Arc::new(AppState {
        todos: Mutex::new(vec![]),
    });

    let api_router = Router::new()
    .route("/hello", get(hello_from_the_server))
    .route("/todos", post(add_todo))
    .with_state(app_state);

    let assets_path = std::env::current_dir().unwrap();

    let router = Router::new()
    .nest("/api", api_router)
    .route("/", get(hello))
    .route("/another-page", get(another_page))
    .nest_service(
        "/assets",
        ServeDir::new(format!("{}/assets", assets_path.to_str().unwrap())),
    );
    let port = "8000";
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("router initialized, now listening on port {}", port);

    axum::serve(listener, router).await.unwrap();

}

async fn hello() -> impl IntoResponse {
    let template = HelloTemplate {};
    HtmlTemplate(template)
}

async fn another_page() -> impl IntoResponse {
    let template = AnotherPageTemplate {};
    HtmlTemplate(template)
}

async fn hello_from_the_server() -> &'static str {
    "Hello!"
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate;

#[derive(Template)]
#[template(path = "another-page.html")]
struct AnotherPageTemplate;

#[derive(Template)]
#[template(path = "todo-list.html")]
struct TodoList {
    todos: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TodoRequest {
    todo: String,
}

async fn add_todo(
    State(state): State<Arc<AppState>>,
    Form(todo): Form<TodoRequest>,
) -> impl IntoResponse {
    let mut lock = state.todos.lock().unwrap();
    lock.push(todo.todo);

    let template = TodoList {
        todos: lock.clone(),
    };

    HtmlTemplate(template)
}

/// A wrapper type that we'll use to encapsulate HTML parsed by askama into valid HTML for axum to serve.
struct HtmlTemplate<T>(T);

/// Allows us to convert Askama HTML templates into valid HTML for axum to serve in the response.
impl<T> IntoResponse for HtmlTemplate<T>
    where
        T: Template,
{
    fn into_response(self) -> Response {
        // Attempt to render the template with askama
        match self.0.render() {
            // If we're able to successfully parse and aggregate the template, serve it
            Ok(html) => Html(html).into_response(),
            // If we're not, return an error or some bit of fallback HTML
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}