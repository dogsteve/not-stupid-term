use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Deserialize)]
pub struct McpRequest {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct McpResponse {
    pub status: String,
    pub result: Option<serde_json::Value>,
}

pub async fn start_mcp_server() {
    let app = Router::new().route("/mcp", post(handle_mcp));
    
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}. Trying random port...", addr, e);
            tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await.unwrap()
        }
    };
    
    println!("MCP Server listening on {}", listener.local_addr().unwrap());
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("MCP Server error: {}", e);
    }
}

async fn handle_mcp(Json(payload): Json<McpRequest>) -> Json<McpResponse> {
    // For MVP, just return a mock response or log the command
    println!("Received MCP command: {}", payload.method);
    
    Json(McpResponse {
        status: "success".into(),
        result: Some(serde_json::json!({ "message": format!("Executed {}", payload.method) })),
    })
}
