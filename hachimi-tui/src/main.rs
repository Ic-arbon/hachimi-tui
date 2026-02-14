mod config;
mod player;
#[macro_use]
mod ui;
mod app;
mod model;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = app::App::new().await?;
    app.run().await
}
