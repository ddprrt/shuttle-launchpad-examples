use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Deserialize, FromRow, Serialize)]
struct Article {
    title: String,
    content: String,
    published_date: String,
}

async fn create_article(
    State(pool): State<PgPool>,
    Json(new_article): Json<Article>,
) -> impl IntoResponse {
    // Insert the new article into the database
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("INSERT INTO articles (title, content, published_date)");

    query_builder.push_values([new_article], |mut b, article| {
        b.push_bind(article.title)
            .push_bind(article.content)
            .push_bind(article.published_date);
    });

    let result = query_builder.build().execute(&pool).await;

    match result {
        Ok(_) => (StatusCode::OK, "Article created".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error creating article: {}", e.to_string()),
        ),
    }
}

async fn get_article(
    Path(article_id): Path<usize>,
    State(pool): State<PgPool>,
) -> Result<Json<Article>, (StatusCode, String)> {
    let query = format!(
        "SELECT title, content, published_date FROM articles WHERE id = {}",
        article_id
    );
    let result = sqlx::query_as(&query);

    let article: Article = result.fetch_one(&pool).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            format!("Article with id {} not found", article_id),
        )
    })?;
    Ok(Json(article))
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(local_uri = "postgres://stefan@localhost:5432/postgres")]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(shuttle_runtime::CustomError::new)?;
    let router = Router::new()
        .route("/articles", post(create_article))
        .route("/articles/:id", get(get_article))
        .with_state(pool);

    Ok(router.into())
}
