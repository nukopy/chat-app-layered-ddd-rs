//! Core domain models for the chat application.

use serde::{Deserialize, Serialize};

use super::{
    error::RoomError,
    value_object::{ClientId, MessageContent, RoomId, Timestamp},
};

/// Default maximum number of participants allowed in a room
pub const DEFAULT_PARTICIPANT_CAPACITY: usize = 10;

/// Default maximum number of messages allowed in a room
pub const DEFAULT_MESSAGE_CAPACITY: usize = 100;

/// Represents a chat room with participants and message history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// Room identifier
    pub id: RoomId,
    /// List of participants currently in the room
    pub participants: Vec<Participant>,
    /// Message history in the room
    pub messages: Vec<ChatMessage>,
    /// Timestamp when the room was created
    pub created_at: Timestamp,
    /// Maximum number of participants allowed (default: 10)
    pub participant_capacity: usize,
    /// Maximum number of messages allowed (default: 100)
    pub message_capacity: usize,
}

impl Room {
    /// Create a new empty room with the given ID and creation timestamp
    pub fn new(id: RoomId, created_at: Timestamp) -> Self {
        Self {
            id,
            participants: Vec::new(),
            messages: Vec::new(),
            created_at,
            participant_capacity: DEFAULT_PARTICIPANT_CAPACITY,
            message_capacity: DEFAULT_MESSAGE_CAPACITY,
        }
    }

    /// Create a new empty room with custom capacities
    pub fn with_capacity(
        id: RoomId,
        created_at: Timestamp,
        participant_capacity: usize,
        message_capacity: usize,
    ) -> Self {
        Self {
            id,
            participants: Vec::new(),
            messages: Vec::new(),
            created_at,
            participant_capacity,
            message_capacity,
        }
    }

    /// Add a participant to the room
    ///
    /// # Errors
    ///
    /// Returns `RoomError::CapacityExceeded` if the room is at full capacity
    pub fn add_participant(&mut self, participant: Participant) -> Result<(), RoomError> {
        if self.participants.len() >= self.participant_capacity {
            return Err(RoomError::CapacityExceeded {
                capacity: self.participant_capacity,
                current: self.participants.len(),
            });
        }
        self.participants.push(participant);
        Ok(())
    }

    /// Remove a participant from the room by ID
    pub fn remove_participant(&mut self, participant_id: &ClientId) {
        self.participants.retain(|p| &p.id != participant_id);
    }

    /// Add a message to the room history
    ///
    /// # Errors
    ///
    /// Returns `RoomError::MessageCapacityExceeded` if the room message history is at full capacity
    pub fn add_message(&mut self, message: ChatMessage) -> Result<(), RoomError> {
        if self.messages.len() >= self.message_capacity {
            return Err(RoomError::MessageCapacityExceeded {
                capacity: self.message_capacity,
                current: self.messages.len(),
            });
        }
        self.messages.push(message);
        Ok(())
    }

    /// Get a participant by ID
    pub fn get_participant(&self, participant_id: &ClientId) -> Option<&Participant> {
        self.participants.iter().find(|p| &p.id == participant_id)
    }
}

/// Represents a participant in a chat room
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    /// Participant identifier (client_id)
    pub id: ClientId,
    /// Timestamp when the participant connected
    pub connected_at: Timestamp,
}

impl Participant {
    /// Create a new participant
    pub fn new(id: ClientId, connected_at: Timestamp) -> Self {
        Self { id, connected_at }
    }
}

/// Represents a chat message in the domain model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Sender's participant ID
    pub from: ClientId,
    /// Message content
    pub content: MessageContent,
    /// Timestamp when the message was sent
    pub timestamp: Timestamp,
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(from: ClientId, content: MessageContent, timestamp: Timestamp) -> Self {
        Self {
            from,
            content,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::factory::RoomIdFactory;

    #[test]
    fn test_room_new() {
        // テスト項目: 新しい Room が空の状態で作成される
        // given (前提条件):
        let room_id = RoomIdFactory::generate().unwrap();
        let created_at = Timestamp::new(1000);

        // when (操作):
        let room = Room::new(room_id.clone(), created_at);

        // then (期待する結果):
        assert_eq!(room.id, room_id);
        assert_eq!(room.participants.len(), 0);
        assert_eq!(room.messages.len(), 0);
        assert_eq!(room.created_at, created_at);
    }

    #[test]
    fn test_room_add_participant() {
        // テスト項目: 参加者を追加できる
        // given (前提条件):
        let mut room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));
        let participant = Participant::new(
            ClientId::new("alice".to_string()).unwrap(),
            Timestamp::new(1000),
        );

        // when (操作):
        let result = room.add_participant(participant.clone());

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(room.participants.len(), 1);
        assert_eq!(
            room.participants[0].id,
            ClientId::new("alice".to_string()).unwrap()
        );
    }

    #[test]
    fn test_room_remove_participant() {
        // テスト項目: 参加者を削除できる
        // given (前提条件):
        let mut room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));
        room.add_participant(Participant::new(
            ClientId::new("alice".to_string()).unwrap(),
            Timestamp::new(1000),
        ))
        .unwrap();
        room.add_participant(Participant::new(
            ClientId::new("bob".to_string()).unwrap(),
            Timestamp::new(2000),
        ))
        .unwrap();

        // when (操作):
        let alice_id = ClientId::new("alice".to_string()).unwrap();
        room.remove_participant(&alice_id);

        // then (期待する結果):
        assert_eq!(room.participants.len(), 1);
        assert_eq!(
            room.participants[0].id,
            ClientId::new("bob".to_string()).unwrap()
        );
    }

    #[test]
    fn test_room_add_message() {
        // テスト項目: メッセージを追加できる
        // given (前提条件):
        let mut room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));
        let message = ChatMessage::new(
            ClientId::new("alice".to_string()).unwrap(),
            MessageContent::new("Hello!".to_string()).unwrap(),
            Timestamp::new(3000),
        );

        // when (操作):
        let result = room.add_message(message.clone());

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(room.messages.len(), 1);
        assert_eq!(
            room.messages[0].from,
            ClientId::new("alice".to_string()).unwrap()
        );
        assert_eq!(
            room.messages[0].content,
            MessageContent::new("Hello!".to_string()).unwrap()
        );
    }

    #[test]
    fn test_room_get_participant() {
        // テスト項目: ID で参加者を取得できる
        // given (前提条件):
        let mut room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));
        let alice_id = ClientId::new("alice".to_string()).unwrap();
        room.add_participant(Participant::new(alice_id.clone(), Timestamp::new(1000)))
            .unwrap();

        // when (操作):
        let participant = room.get_participant(&alice_id);

        // then (期待する結果):
        assert!(participant.is_some());
        assert_eq!(participant.unwrap().id, alice_id);
    }

    #[test]
    fn test_room_get_nonexistent_participant() {
        // テスト項目: 存在しない参加者は None が返される
        // given (前提条件):
        let room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));

        // when (操作):
        let alice_id = ClientId::new("alice".to_string()).unwrap();
        let participant = room.get_participant(&alice_id);

        // then (期待する結果):
        assert!(participant.is_none());
    }

    #[test]
    fn test_room_participant_capacity_exceeded() {
        // テスト項目: 参加者数が上限に達したらエラーが返される
        // given (前提条件):
        let mut room = Room::with_capacity(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(0),
            2, // participant_capacity
            100,
        );

        // when (操作):
        room.add_participant(Participant::new(
            ClientId::new("alice".to_string()).unwrap(),
            Timestamp::new(1000),
        ))
        .unwrap();
        room.add_participant(Participant::new(
            ClientId::new("bob".to_string()).unwrap(),
            Timestamp::new(2000),
        ))
        .unwrap();

        let result = room.add_participant(Participant::new(
            ClientId::new("charlie".to_string()).unwrap(),
            Timestamp::new(3000),
        ));

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RoomError::CapacityExceeded {
                capacity: 2,
                current: 2
            }
        );
        assert_eq!(room.participants.len(), 2);
    }

    #[test]
    fn test_room_message_capacity_exceeded() {
        // テスト項目: メッセージ数が上限に達したらエラーが返される
        // given (前提条件):
        let mut room = Room::with_capacity(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(0),
            10,
            2, // message_capacity
        );

        // when (操作):
        room.add_message(ChatMessage::new(
            ClientId::new("alice".to_string()).unwrap(),
            MessageContent::new("Hello!".to_string()).unwrap(),
            Timestamp::new(1000),
        ))
        .unwrap();
        room.add_message(ChatMessage::new(
            ClientId::new("bob".to_string()).unwrap(),
            MessageContent::new("Hi!".to_string()).unwrap(),
            Timestamp::new(2000),
        ))
        .unwrap();

        let result = room.add_message(ChatMessage::new(
            ClientId::new("charlie".to_string()).unwrap(),
            MessageContent::new("Hey!".to_string()).unwrap(),
            Timestamp::new(3000),
        ));

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RoomError::MessageCapacityExceeded {
                capacity: 2,
                current: 2
            }
        );
        assert_eq!(room.messages.len(), 2);
    }

    #[test]
    fn test_room_default_capacities() {
        // テスト項目: デフォルトの上限値が正しく設定される
        // given (前提条件):
        let room = Room::new(RoomIdFactory::generate().unwrap(), Timestamp::new(0));

        // then (期待する結果):
        assert_eq!(room.participant_capacity, DEFAULT_PARTICIPANT_CAPACITY);
        assert_eq!(room.message_capacity, DEFAULT_MESSAGE_CAPACITY);
    }
}
