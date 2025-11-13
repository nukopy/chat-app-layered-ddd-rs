//! Conversion logic between DTOs and domain entities.

use crate::domain::{
    entity,
    value_object::{ClientId, MessageContent, Timestamp},
};
use crate::infrastructure::dto::websocket as dto;

// ========================================
// DTO → Domain Entity
// ========================================

impl From<dto::ChatMessage> for entity::ChatMessage {
    fn from(dto: dto::ChatMessage) -> Self {
        Self {
            from: ClientId::new(dto.client_id).expect("ClientId should be valid in DTO"),
            content: MessageContent::new(dto.content)
                .expect("MessageContent should be valid in DTO"),
            timestamp: Timestamp::new(dto.timestamp),
        }
    }
}

impl From<dto::ParticipantInfo> for entity::Participant {
    fn from(dto: dto::ParticipantInfo) -> Self {
        Self {
            id: ClientId::new(dto.client_id).expect("ClientId should be valid in DTO"),
            connected_at: Timestamp::new(dto.connected_at),
        }
    }
}

// ========================================
// Domain Entity → DTO
// ========================================

impl From<entity::ChatMessage> for dto::ChatMessage {
    fn from(model: entity::ChatMessage) -> Self {
        Self {
            r#type: dto::MessageType::Chat,
            client_id: model.from.into_string(),
            content: model.content.into_string(),
            timestamp: model.timestamp.value(),
        }
    }
}

impl From<entity::Participant> for dto::ParticipantInfo {
    fn from(model: entity::Participant) -> Self {
        Self {
            client_id: model.id.into_string(),
            connected_at: model.connected_at.value(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dto_chat_message_to_domain() {
        // テスト項目: DTO の ChatMessage がドメインエンティティに変換される
        // given (前提条件):
        let dto_msg = dto::ChatMessage {
            r#type: dto::MessageType::Chat,
            client_id: "alice".to_string(),
            content: "Hello!".to_string(),
            timestamp: 1000,
        };

        // when (操作):
        let domain_msg: entity::ChatMessage = dto_msg.into();

        // then (期待する結果):
        assert_eq!(domain_msg.from, ClientId::new("alice".to_string()).unwrap());
        assert_eq!(
            domain_msg.content,
            MessageContent::new("Hello!".to_string()).unwrap()
        );
        assert_eq!(domain_msg.timestamp, Timestamp::new(1000));
    }

    #[test]
    fn test_domain_chat_message_to_dto() {
        // テスト項目: ドメインエンティティの ChatMessage が DTO に変換される
        // given (前提条件):
        let domain_msg = entity::ChatMessage {
            from: ClientId::new("bob".to_string()).unwrap(),
            content: MessageContent::new("Hi!".to_string()).unwrap(),
            timestamp: Timestamp::new(2000),
        };

        // when (操作):
        let dto_msg: dto::ChatMessage = domain_msg.into();

        // then (期待する結果):
        assert_eq!(dto_msg.client_id, "bob");
        assert_eq!(dto_msg.content, "Hi!");
        assert_eq!(dto_msg.timestamp, 2000);
        assert!(matches!(dto_msg.r#type, dto::MessageType::Chat));
    }

    #[test]
    fn test_dto_participant_to_domain() {
        // テスト項目: DTO の ParticipantInfo がドメインエンティティに変換される
        // given (前提条件):
        let dto_participant = dto::ParticipantInfo {
            client_id: "alice".to_string(),
            connected_at: 1000,
        };

        // when (操作):
        let domain_participant: entity::Participant = dto_participant.into();

        // then (期待する結果):
        assert_eq!(
            domain_participant.id,
            ClientId::new("alice".to_string()).unwrap()
        );
        assert_eq!(domain_participant.connected_at, Timestamp::new(1000));
    }

    #[test]
    fn test_domain_participant_to_dto() {
        // テスト項目: ドメインエンティティの Participant が DTO に変換される
        // given (前提条件):
        let domain_participant = entity::Participant {
            id: ClientId::new("bob".to_string()).unwrap(),
            connected_at: Timestamp::new(2000),
        };

        // when (操作):
        let dto_participant: dto::ParticipantInfo = domain_participant.into();

        // then (期待する結果):
        assert_eq!(dto_participant.client_id, "bob");
        assert_eq!(dto_participant.connected_at, 2000);
    }
}
