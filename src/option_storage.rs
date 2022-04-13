use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serenity::model::id::UserId;
use sqlx::mysql::MySqlPool;

use crate::tts;
use crate::tts::voice_text::{VoiceTextFormat, VoiceTextOptions, VoiceTextSpeaker};

const DEFAULT_OPTIONS: tts::Options = tts::Options::VoiceTextOptions(VoiceTextOptions {
    speaker: VoiceTextSpeaker::Show,
    format: VoiceTextFormat::Wav,
    emotion: None,
    emotion_level: 2,
    pitch: 100,
    speed: 100,
    volume: 100,
});

pub struct OptionStorage {
    cache: HashMap<u64, tts::Options>,
    pool: MySqlPool,
}

impl OptionStorage {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = MySqlPool::connect(database_url).await?;
        let records = sqlx::query!("SELECT user_id, options FROM options")
            .fetch_all(&pool)
            .await?;
        Ok(Self {
            cache: HashMap::from_iter(records.into_iter().map(|r| {
                (
                    r.user_id,
                    serde_json::from_value(r.options.unwrap()).unwrap(),
                )
            })),
            pool,
        })
    }

    pub fn get(&self, user_id: &UserId) -> tts::Options {
        self.cache
            .get(&user_id.0)
            .cloned()
            .unwrap_or(DEFAULT_OPTIONS)
    }

    pub async fn set(&mut self, user_id: &UserId, options: tts::Options) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
REPLACE INTO options (user_id, options)
VALUES (?, ?)
            "#,
            user_id.0,
            serde_json::to_string(&options)?
        )
        .execute(&self.pool)
        .await?;
        self.cache.insert(user_id.0, options);
        Ok(())
    }
}
