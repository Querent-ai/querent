use actors::ActorExitStatus;
use async_trait::async_trait;
use serde::Serialize;
use std::fmt;
use std::time::Duration;

use crate::{Source, SourceContext};

// Note: This is just an example of a custom source.
// It is not used in the current implementation.
// It is kept here for reference.
//
#[derive(Default, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FileSourceCounters {
    pub previous_offset: u64,
    pub current_offset: u64,
    pub num_lines_processed: u64,
}

pub struct FileSource {
    source_id: String,
    counters: FileSourceCounters,
}

impl fmt::Debug for FileSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileSource {{ source_id: {} }}", self.source_id)
    }
}

#[async_trait]
impl Source for FileSource {
    async fn emit_events(&mut self, _ctx: &SourceContext) -> Result<Duration, ActorExitStatus> {
        self.counters.previous_offset = 1030;
        self.counters.current_offset += 1030;
        self.counters.num_lines_processed += 4;
        Err(ActorExitStatus::Success)
    }

    fn name(&self) -> String {
        format!("FileSource{{source_id={}}}", self.source_id)
    }

    fn observable_state(&self) -> serde_json::Value {
        serde_json::to_value(&self.counters).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use actors::Universe;

    use crate::{file_source::FileSource, source::SourceActor};

    #[tokio::test]
    async fn test_file_source() -> anyhow::Result<()> {
        let universe = Universe::with_accelerated_time();
        let file_source = FileSource {
            source_id: "test_file_source".to_string(),
            counters: Default::default(),
        };
        let file_source_actor = SourceActor {
            source: Box::new(file_source),
        };
        let (_file_source_mailbox, file_source_handle) =
            universe.spawn_builder().spawn(file_source_actor);
        let (actor_termination, counters) = file_source_handle.join().await;
        assert!(actor_termination.is_success());
        assert_eq!(
            counters,
            serde_json::json!({
                "previous_offset": 1030u64,
                "current_offset": 1030u64,
                "num_lines_processed": 4u32
            })
        );
        Ok(())
    }
}
