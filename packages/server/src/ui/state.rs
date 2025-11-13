//! Server state and connection management.

use std::sync::Arc;

use crate::usecase::{
    ConnectParticipantUseCase, DisconnectParticipantUseCase, GetRoomDetailUseCase,
    GetRoomStateUseCase, GetRoomsUseCase, SendMessageUseCase,
};

/// Shared application state
///
/// AppState は UseCase のみを保持します。
/// Repository や MessagePusher は UseCase が内部で保持しており、
/// ハンドラーからは UseCase を通じてのみアクセスします。
pub struct AppState {
    /// ConnectParticipantUseCase（参加者接続のユースケース）
    pub connect_participant_usecase: Arc<ConnectParticipantUseCase>,
    /// DisconnectParticipantUseCase（参加者切断のユースケース）
    pub disconnect_participant_usecase: Arc<DisconnectParticipantUseCase>,
    /// SendMessageUseCase（メッセージ送信のユースケース）
    pub send_message_usecase: Arc<SendMessageUseCase>,
    /// GetRoomStateUseCase（ルーム状態取得のユースケース）
    pub get_room_state_usecase: Arc<GetRoomStateUseCase>,
    /// GetRoomsUseCase（ルーム一覧取得のユースケース）
    pub get_rooms_usecase: Arc<GetRoomsUseCase>,
    /// GetRoomDetailUseCase（ルーム詳細取得のユースケース）
    pub get_room_detail_usecase: Arc<GetRoomDetailUseCase>,
}
