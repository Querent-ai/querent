use crate::UpdateResult;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateResultEvent(UpdateResult);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct CheckUpdateEvent;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, specta::Type, tauri_specta::Event)]
pub struct PinnedFromWindowEvent {
    pinned: bool,
}

impl PinnedFromWindowEvent {
    pub fn pinned(&self) -> &bool {
        &self.pinned
    }
}
