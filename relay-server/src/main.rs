use anyhow::Result;
use sankey_copier_relay_server::bootstrap;

#[tokio::main]
async fn main() -> Result<()> {
    // Bootstrap the application (setup logging, DB, ZMQ tasks, API router)
    let app = bootstrap::setup().await?;

    // Start HTTPS server
    tracing::info!("HTTPS server listening on https://{}", app.bind_address);

    axum_server::bind_rustls(app.socket_addr, app.tls_config)
        .serve(app.router.into_make_service())
        .await?;

    Ok(())
}
