mod api;
mod config;
mod player;
mod ui;

mod app;
mod model;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("hachimi_tui=info".parse()?),
        )
        .init();

    let mut app = app::App::new().await?;
    app.run().await
}
