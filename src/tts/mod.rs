pub mod voice_text;
pub mod voice_vox;

use std::convert::From;
use std::fmt;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use self::voice_text::{VoiceTextClient, VoiceTextOptions, VoiceTextOptionsBuilder};
use self::voice_vox::{VoiceVoxClient, VoiceVoxOptions, VoiceVoxOptionsBuilder};

#[derive(Display, EnumIter, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Engine {
    VoiceText,
    VoiceVox,
}

#[derive(Display, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Preset {
    Takuya,
    Munou,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Options {
    VoiceTextOptions(VoiceTextOptions),
    VoiceVoxOptions(VoiceVoxOptions),
}

impl From<Preset> for Options {
    fn from(preset: Preset) -> Self {
        match preset {
            Preset::Takuya => Options::VoiceTextOptions(
                VoiceTextOptionsBuilder::default()
                    .speaker("show".try_into().unwrap())
                    .build()
                    .unwrap(),
            ),
            Preset::Munou => Options::VoiceTextOptions(
                VoiceTextOptionsBuilder::default()
                    .speaker("show".try_into().unwrap())
                    .pitch(150)
                    .build()
                    .unwrap(),
            ),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    voice_text_client: VoiceTextClient,
    voice_vox_client: VoiceVoxClient,
}

impl Client {
    pub fn new(voice_text_api_key: String, voice_vox_api_key: String) -> Self {
        Self {
            voice_text_client: VoiceTextClient::new(voice_text_api_key),
            voice_vox_client: VoiceVoxClient::new(voice_vox_api_key),
        }
    }

    pub async fn request(
        &self,
        text: impl fmt::Display,
        options: &Options,
    ) -> anyhow::Result<Vec<u8>> {
        match options {
            Options::VoiceTextOptions(options) => {
                self.voice_text_client.request(text, &options).await
            }
            Options::VoiceVoxOptions(options) => {
                self.voice_vox_client.request(text, &options).await
            }
        }
    }
}
