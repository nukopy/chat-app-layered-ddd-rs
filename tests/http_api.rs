//! HTTP API integration tests.
//!
//! Tests for REST API endpoints (health check, room list, room details).

mod fixtures;
use fixtures::TestServer;

#[tokio::test]
async fn test_health_endpoint() {
    // テスト項目: /api/health エンドポイントが正常に動作する
    // given (前提条件):
    let port = 19080;
    let server = TestServer::start(port);
    let client = reqwest::Client::new();

    // when (操作):
    let response = client
        .get(format!("{}/api/health", server.base_url()))
        .send()
        .await
        .expect("Failed to send request");

    // then (期待する結果):
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_rooms_list_endpoint() {
    // テスト項目: /api/rooms エンドポイントがルーム一覧を返す
    // given (前提条件):
    let port = 19081;
    let server = TestServer::start(port);
    let client = reqwest::Client::new();

    // when (操作):
    let response = client
        .get(format!("{}/api/rooms", server.base_url()))
        .send()
        .await
        .expect("Failed to send request");

    // then (期待する結果):
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body.is_array(), "Response should be an array");

    // デフォルトでは1つのルーム（"default"）が存在する
    let rooms = body.as_array().unwrap();
    assert_eq!(rooms.len(), 1);

    // ルームの構造を確認
    let room = &rooms[0];
    assert_eq!(room["id"], "default");
    assert!(room["participants"].is_array());
    assert!(room["created_at"].is_string());
}

#[tokio::test]
async fn test_room_detail_endpoint_success() {
    // テスト項目: /api/rooms/:room_id エンドポイントが正常にルーム詳細を返す
    // given (前提条件):
    let port = 19082;
    let server = TestServer::start(port);
    let client = reqwest::Client::new();

    // when (操作):
    let response = client
        .get(format!("{}/api/rooms/default", server.base_url()))
        .send()
        .await
        .expect("Failed to send request");

    // then (期待する結果):
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["id"], "default");
    assert!(body["participants"].is_array());
    assert!(body["created_at"].is_string());

    // participants の各要素が client_id と connected_at を持つ
    let participants = body["participants"].as_array().unwrap();
    for participant in participants {
        assert!(participant["client_id"].is_string());
        assert!(participant["connected_at"].is_string());
    }
}

#[tokio::test]
async fn test_room_detail_endpoint_not_found() {
    // テスト項目: /api/rooms/:room_id エンドポイントが存在しないルームに対して404を返す
    // given (前提条件):
    let port = 19083;
    let server = TestServer::start(port);
    let client = reqwest::Client::new();

    // when (操作):
    let response = client
        .get(format!("{}/api/rooms/nonexistent", server.base_url()))
        .send()
        .await
        .expect("Failed to send request");

    // then (期待する結果):
    assert_eq!(response.status(), 404);
}
