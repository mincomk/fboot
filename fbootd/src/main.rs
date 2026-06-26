use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,fbootd=debug,tower_http=info")))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let config = fbootd::config::Config::from_env();

    let stdio_mcp = std::env::var("FBOOTD_MCP_STDIO")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if stdio_mcp {
        tracing::info!("starting fbootd MCP server over stdio");
        let state = fbootd::build_state(config).await?;
        fbootd::mcp::serve_stdio(state).await?;
        return Ok(());
    }

    tracing::info!(?config.api_addr, "starting fbootd");

    let state = fbootd::build_state(config.clone()).await?;

    fbootd::services::spawn_all(state.clone()).await?;
    fbootd::tasks::spawn_all(state.clone());

    if let Some(mcp_addr) = config.mcp_http_addr {
        let mcp_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = fbootd::mcp::serve_http(mcp_state, mcp_addr).await {
                tracing::error!("mcp server error: {e}");
            }
        });
    }

    let app = fbootd::api::router(state);
    let listener = tokio::net::TcpListener::bind(config.api_addr).await?;
    tracing::info!("API listening on {}", config.api_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
