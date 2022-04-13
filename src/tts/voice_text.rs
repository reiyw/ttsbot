use serde::{Deserialize, Serialize};
use std::fmt;
use std::string::ToString;
use strum::{Display, EnumIter, EnumString};

#[derive(Debug)]
pub struct VoiceTextClient {
    api_key: String,
    client: reqwest::Client,
}

impl VoiceTextClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn request(
        &self,
        text: impl fmt::Display,
        options: &VoiceTextOptions,
    ) -> anyhow::Result<Vec<u8>> {
        let mut params = vec![
            ("text", text.to_string()),
            ("speaker", options.speaker.to_string()),
            ("format", options.format.to_string()),
            ("pitch", options.pitch.to_string()),
            ("speed", options.speed.to_string()),
            ("volume", options.volume.to_string()),
        ];
        if let Some(ref emotion) = options.emotion {
            params.push(("emotion", emotion.to_string()));
            params.push(("emotion_level", options.emotion_level.to_string()))
        }

        let resp = self
            .client
            .post("https://api.voicetext.jp/v1/tts")
            .basic_auth(&self.api_key, None as Option<&str>)
            .form(&params)
            .send()
            .await?;
        Ok(resp.bytes().await?.to_vec())
    }
}

#[derive(Builder, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct VoiceTextOptions {
    pub speaker: VoiceTextSpeaker,
    #[builder(default = "VoiceTextFormat::Wav")]
    pub format: VoiceTextFormat,
    #[builder(default = "None")]
    pub emotion: Option<VoiceTextEmotion>,
    #[builder(default = "2")]
    pub emotion_level: u8,
    #[builder(default = "100")]
    pub pitch: u8,
    #[builder(default = "100")]
    pub speed: u16,
    #[builder(default = "100")]
    pub volume: u8,
}

impl VoiceTextOptionsBuilder {
    fn validate(&self) -> Result<(), String> {
        if let Some(ref emotion) = self.emotion {
            if let Some(ref speaker) = self.speaker {
                if emotion.is_some() && *speaker == VoiceTextSpeaker::Show {
                    return Err(
                        "emotion can be used when speaker is haruka, hikari, takeru santa, or bear"
                            .to_string(),
                    );
                }
            }
        }

        if let Some(emotion_level) = self.emotion_level {
            if emotion_level < 1 || emotion_level > 4 {
                return Err("Bad emotion_level, must be 1 <= emotion_level <= 4".to_string());
            }
        }

        if let Some(pitch) = self.pitch {
            if pitch < 50 || pitch > 200 {
                return Err("Bad pitch, must be 50 <= pitch <= 200".to_string());
            }
        }

        if let Some(speed) = self.speed {
            if speed < 50 || speed > 400 {
                return Err("Bad speed, must be 50 <= speed <= 400".to_string());
            }
        }

        if let Some(volume) = self.volume {
            if volume < 50 || volume > 200 {
                return Err("Bad volume, must be 50 <= volume <= 200".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Display, EnumIter, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum VoiceTextSpeaker {
    Show,
    Haruka,
    Hikari,
    Takeru,
    Santa,
    Bear,
}

#[derive(Clone, Debug, Deserialize, Display, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum VoiceTextFormat {
    Wav,
    Ogg,
    Mp3,
}

#[derive(Clone, Debug, Deserialize, Display, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum VoiceTextEmotion {
    Happiness,
    Anger,
    Sadness,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_builder() {
        let opt = VoiceTextOptionsBuilder::default()
            .speaker("show".try_into().unwrap())
            .build()
            .unwrap();
        assert_eq!(
            opt,
            VoiceTextOptions {
                speaker: VoiceTextSpeaker::Show,
                format: VoiceTextFormat::Wav,
                emotion: None,
                emotion_level: 2,
                pitch: 100,
                speed: 100,
                volume: 100
            }
        );

        let err = VoiceTextOptionsBuilder::default()
            .speaker("show".try_into().unwrap())
            .emotion(Some("happiness".try_into().unwrap()))
            .build()
            .unwrap_err();
        assert_eq!(
            &err.to_string(),
            "emotion can be used when speaker is haruka, hikari, takeru santa, or bear"
        );

        let err = VoiceTextOptionsBuilder::default()
            .speaker("show".try_into().unwrap())
            .pitch(49)
            .build()
            .unwrap_err();
        assert_eq!(&err.to_string(), "Bad pitch, must be 50 <= pitch <= 200");

        let opt = VoiceTextOptionsBuilder::default()
            .speaker("haruka".try_into().unwrap())
            .format("mp3".try_into().unwrap())
            .emotion(Some("happiness".try_into().unwrap()))
            .emotion_level(4)
            .pitch(50)
            .speed(400)
            .volume(200)
            .build()
            .unwrap();
        assert_eq!(
            opt,
            VoiceTextOptions {
                speaker: VoiceTextSpeaker::Haruka,
                format: VoiceTextFormat::Mp3,
                emotion: Some(VoiceTextEmotion::Happiness),
                emotion_level: 4,
                pitch: 50,
                speed: 400,
                volume: 200
            }
        );
    }
}
