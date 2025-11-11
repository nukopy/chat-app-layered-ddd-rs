//! Message formatting utilities for client display.

#![allow(dead_code)]

use crate::{
    common::time::timestamp_to_jst_rfc3339, infrastructure::dto::websocket::ParticipantInfo,
};

/// Message formatter for client display
pub struct MessageFormatter;

impl MessageFormatter {
    /// Format the room-connected message showing all participants
    ///
    /// # Arguments
    ///
    /// * `participants` - List of participants in the room
    /// * `current_client_id` - The current client's ID (to mark as "me")
    ///
    /// # Returns
    ///
    /// A formatted string with participant list
    pub fn format_room_connected(
        participants: &[ParticipantInfo],
        current_client_id: &str,
    ) -> String {
        let mut output = String::new();
        output.push_str("\n\n============================================================\n");
        output.push_str("Participants:\n");

        if participants.is_empty() {
            output.push_str("(No participants)\n");
        } else {
            for participant in participants {
                let is_me = participant.client_id == current_client_id;
                let me_suffix = if is_me { " (me)" } else { "" };
                let timestamp_str = timestamp_to_jst_rfc3339(participant.connected_at);
                output.push_str(&format!(
                    "{}{} - entered at {}\n",
                    participant.client_id, me_suffix, timestamp_str
                ));
            }
        }

        output.push_str("============================================================\n\n");
        output
    }

    /// Format a participant-joined notification
    ///
    /// # Arguments
    ///
    /// * `client_id` - The ID of the participant who joined
    /// * `connected_at` - Unix timestamp when the participant connected (milliseconds)
    ///
    /// # Returns
    ///
    /// A formatted string with the join notification
    pub fn format_participant_joined(client_id: &str, connected_at: i64) -> String {
        let timestamp_str = timestamp_to_jst_rfc3339(connected_at);
        format!("\n+ {} entered at {}\n", client_id, timestamp_str)
    }

    /// Format a participant-left notification
    ///
    /// # Arguments
    ///
    /// * `client_id` - The ID of the participant who left
    /// * `disconnected_at` - Unix timestamp when the participant disconnected (milliseconds)
    ///
    /// # Returns
    ///
    /// A formatted string with the leave notification
    pub fn format_participant_left(client_id: &str, disconnected_at: i64) -> String {
        let timestamp_str = timestamp_to_jst_rfc3339(disconnected_at);
        format!("\n- {} left at {}\n", client_id, timestamp_str)
    }

    /// Format a chat message
    ///
    /// # Arguments
    ///
    /// * `from` - The client ID of the sender
    /// * `content` - The message content
    /// * `sent_at` - Unix timestamp when the message was sent (milliseconds)
    ///
    /// # Returns
    ///
    /// A formatted string with the chat message
    pub fn format_chat_message(from: &str, content: &str, sent_at: i64) -> String {
        let timestamp_str = timestamp_to_jst_rfc3339(sent_at);
        format!(
            "\n\n------------------------------------------------------------\n\
             @{}: {}\n\
             sent at {}\n\
             ------------------------------------------------------------\n\n",
            from, content, timestamp_str
        )
    }

    /// Format a confirmation message after sending
    ///
    /// # Arguments
    ///
    /// * `sent_at` - Unix timestamp when the message was sent (milliseconds)
    ///
    /// # Returns
    ///
    /// A formatted string with the sent confirmation
    pub fn format_sent_confirmation(sent_at: i64) -> String {
        let timestamp_str = timestamp_to_jst_rfc3339(sent_at);
        format!("sent at {}\n", timestamp_str)
    }

    /// Format a binary message notification
    ///
    /// # Arguments
    ///
    /// * `byte_count` - The number of bytes received
    ///
    /// # Returns
    ///
    /// A formatted string with the binary data notification
    pub fn format_binary_message(byte_count: usize) -> String {
        format!("\n← Received {} bytes of binary data\n", byte_count)
    }

    /// Format a raw text message (when parsing fails)
    ///
    /// # Arguments
    ///
    /// * `text` - The raw text received
    ///
    /// # Returns
    ///
    /// A formatted string with the raw message
    pub fn format_raw_message(text: &str) -> String {
        format!("\n← Received: {}\n", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_room_connected_with_empty_participants() {
        // テスト項目: 参加者が空の場合、適切なメッセージが表示される
        // given (前提条件):
        let participants = vec![];
        let current_client_id = "alice";

        // when (操作):
        let result = MessageFormatter::format_room_connected(&participants, current_client_id);

        // then (期待する結果):
        assert!(result.contains("Participants:"));
        assert!(result.contains("(No participants)"));
        assert!(result.contains("============================================================"));
    }

    #[test]
    fn test_format_room_connected_with_single_participant() {
        // テスト項目: 単一参加者の場合、正しくフォーマットされる
        // given (前提条件):
        let participants = vec![ParticipantInfo {
            client_id: "alice".to_string(),
            connected_at: 1672498800000,
        }];
        let current_client_id = "alice";

        // when (操作):
        let result = MessageFormatter::format_room_connected(&participants, current_client_id);

        // then (期待する結果):
        assert!(result.contains("alice (me)"));
        assert!(result.contains("entered at"));
        assert!(result.contains("2023-01-01"));
    }

    #[test]
    fn test_format_room_connected_with_multiple_participants() {
        // テスト項目: 複数参加者の場合、全員が表示され自分にはマークが付く
        // given (前提条件):
        let participants = vec![
            ParticipantInfo {
                client_id: "alice".to_string(),
                connected_at: 1672498800000,
            },
            ParticipantInfo {
                client_id: "bob".to_string(),
                connected_at: 1672498900000,
            },
        ];
        let current_client_id = "alice";

        // when (操作):
        let result = MessageFormatter::format_room_connected(&participants, current_client_id);

        // then (期待する結果):
        assert!(result.contains("alice (me)"));
        assert!(result.contains("bob - entered at"));
        assert!(!result.contains("bob (me)"));
    }

    #[test]
    fn test_format_participant_joined() {
        // テスト項目: 参加者参加通知が正しくフォーマットされる
        // given (前提条件):
        let client_id = "bob";
        let connected_at = 1672498800000;

        // when (操作):
        let result = MessageFormatter::format_participant_joined(client_id, connected_at);

        // then (期待する結果):
        assert!(result.contains("+ bob"));
        assert!(result.contains("entered at"));
        assert!(result.contains("2023-01-01"));
    }

    #[test]
    fn test_format_participant_left() {
        // テスト項目: 参加者退出通知が正しくフォーマットされる
        // given (前提条件):
        let client_id = "charlie";
        let disconnected_at = 1672498800000;

        // when (操作):
        let result = MessageFormatter::format_participant_left(client_id, disconnected_at);

        // then (期待する結果):
        assert!(result.contains("- charlie"));
        assert!(result.contains("left at"));
        assert!(result.contains("2023-01-01"));
    }

    #[test]
    fn test_format_chat_message() {
        // テスト項目: チャットメッセージが正しくフォーマットされる
        // given (前提条件):
        let from = "alice";
        let content = "Hello, world!";
        let sent_at = 1672498800000;

        // when (操作):
        let result = MessageFormatter::format_chat_message(from, content, sent_at);

        // then (期待する結果):
        assert!(result.contains("@alice:"));
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("sent at"));
        assert!(result.contains("2023-01-01"));
        assert!(result.contains("------------------------------------------------------------"));
    }

    #[test]
    fn test_format_sent_confirmation() {
        // テスト項目: 送信確認メッセージが正しくフォーマットされる
        // given (前提条件):
        let sent_at = 1672498800000;

        // when (操作):
        let result = MessageFormatter::format_sent_confirmation(sent_at);

        // then (期待する結果):
        assert!(result.contains("sent at"));
        assert!(result.contains("2023-01-01"));
    }

    #[test]
    fn test_format_binary_message() {
        // テスト項目: バイナリメッセージ通知が正しくフォーマットされる
        // given (前提条件):
        let byte_count = 1024;

        // when (操作):
        let result = MessageFormatter::format_binary_message(byte_count);

        // then (期待する結果):
        assert!(result.contains("1024 bytes"));
        assert!(result.contains("Received"));
    }

    #[test]
    fn test_format_raw_message() {
        // テスト項目: 生メッセージが正しくフォーマットされる
        // given (前提条件):
        let text = "unknown message format";

        // when (操作):
        let result = MessageFormatter::format_raw_message(text);

        // then (期待する結果):
        assert!(result.contains("unknown message format"));
        assert!(result.contains("Received:"));
    }
}
