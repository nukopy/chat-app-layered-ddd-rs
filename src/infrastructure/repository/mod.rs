//! Repository パターンの実装
//!
//! ドメイン層が定義する Repository trait の具体的な実装を提供します。
//! UseCase 層は trait（ドメイン層）に依存し、この実装に直接依存しません（依存性の逆転）。

pub mod inmemory;

pub use inmemory::InMemoryRoomRepository;
