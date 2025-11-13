//! UseCase 層
//!
//! ビジネスロジックを実装するレイヤー。
//! UI 層から呼び出され、Domain 層を操作します。

pub mod connect_participant;
pub mod disconnect_participant;
pub mod error;
pub mod get_room_detail;
pub mod get_room_state;
pub mod get_rooms;
pub mod send_message;

pub use connect_participant::ConnectParticipantUseCase;
pub use disconnect_participant::DisconnectParticipantUseCase;
pub use error::{ConnectError, SendMessageError};
pub use get_room_detail::{GetRoomDetailError, GetRoomDetailUseCase};
pub use get_room_state::GetRoomStateUseCase;
pub use get_rooms::GetRoomsUseCase;
pub use send_message::SendMessageUseCase;
