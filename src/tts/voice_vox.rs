use serde::{Deserialize, Serialize};
use std::fmt;
use std::string::ToString;
use strum::{Display, EnumIter, EnumString};

#[derive(Debug)]
pub struct VoiceVoxClient {
    api_key: String,
    client: reqwest::Client,
}

impl VoiceVoxClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn request(
        &self,
        text: impl fmt::Display,
        options: &VoiceVoxOptions,
    ) -> anyhow::Result<Vec<u8>> {
        let query = vec![
            ("text", text.to_string()),
            ("key", self.api_key.clone()),
            ("speaker", (options.speaker.clone() as u8).to_string()),
            ("pitch", options.pitch.to_string()),
            ("intonationScale", options.intonation_scale.to_string()),
            ("speed", options.speed.to_string()),
        ];

        let resp = self
            .client
            .get("https://api.su-shiki.com/v2/voicevox/audio")
            .query(&query)
            .send()
            .await?;
        Ok(resp.bytes().await?.to_vec())
    }
}

#[derive(Builder, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct VoiceVoxOptions {
    speaker: VoiceVoxSpeaker,
    #[builder(default = "0.0")]
    pitch: f64,
    #[builder(default = "1.0")]
    intonation_scale: f64,
    #[builder(default = "1.0")]
    speed: f64,
}

#[derive(Clone, Debug, Deserialize, Display, EnumIter, EnumString, PartialEq, Serialize)]
pub enum VoiceVoxSpeaker {
    四国めたん = 2,
    四国めたんあまあま = 0,
    四国めたんツンツン = 6,
    四国めたんセクシー = 4,
    ずんだもん = 3,
    ずんだもんあまあま = 1,
    ずんだもんツンツン = 7,
    ずんだもんセクシー = 5,
    春日部つむぎ = 8,
    雨晴はう = 10,
    波音リツ = 9,
    玄野武宏 = 11,
    白上虎太郎 = 12,
    青山龍星 = 13,
    冥鳴ひまり = 14,
    九州そら = 16,
    九州そらあまあま = 15,
    九州そらツンツン = 18,
    九州そらセクシー = 17,
    九州そらささやき = 19,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_speaker() {
        assert_eq!(
            VoiceVoxSpeaker::try_from("四国めたん").unwrap(),
            VoiceVoxSpeaker::四国めたん
        );
        assert_eq!(VoiceVoxSpeaker::九州そら as u8, 16);
    }
}
