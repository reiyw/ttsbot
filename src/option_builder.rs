use anyhow::Context as _;

use crate::tts::voice_text::{VoiceTextOptions, VoiceTextOptionsBuilder};
use crate::tts::voice_vox::{VoiceVoxOptions, VoiceVoxOptionsBuilder};

pub fn build_voice_text_options<A, S>(args: A) -> anyhow::Result<VoiceTextOptions>
where
    A: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut builder = VoiceTextOptionsBuilder::default();
    for arg in args.into_iter() {
        let mut it = arg.as_ref().split('=');
        let key = it
            .next()
            .context(r#"Each option must be in the form "key=value""#)?;
        let value = it
            .next()
            .context(r#"Each option must be in the form "key=value""#)?;
        match key {
            "speaker" => {
                builder.speaker(value.try_into()?);
            }
            "emotion" => {
                builder.emotion(Some(value.try_into()?));
            }
            "emotion_level" => {
                builder.emotion_level(value.parse()?);
            }
            "pitch" => {
                builder.pitch(value.parse()?);
            }
            "speed" => {
                builder.speed(value.parse()?);
            }
            _ => {}
        }
    }
    let options = builder.build()?;
    Ok(options)
}

pub fn build_voice_vox_options<A, S>(args: A) -> anyhow::Result<VoiceVoxOptions>
where
    A: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut builder = VoiceVoxOptionsBuilder::default();
    for arg in args.into_iter() {
        let mut it = arg.as_ref().split('=');
        let key = it
            .next()
            .context(r#"Each option must be in the form "key=value""#)?;
        let value = it
            .next()
            .context(r#"Each option must be in the form "key=value""#)?;
        match key {
            "speaker" => {
                builder.speaker(value.try_into()?);
            }
            "pitch" => {
                builder.pitch(value.parse()?);
            }
            "intonationScale" => {
                builder.intonation_scale(value.parse()?);
            }
            "speed" => {
                builder.speed(value.parse()?);
            }
            _ => {}
        }
    }
    let options = builder.build()?;
    Ok(options)
}
