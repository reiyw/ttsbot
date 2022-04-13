use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Parser;
use dotenv::dotenv;
use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::RwLock;

use serenity::model::id::ChannelId;
use serenity::model::id::GuildId;
use serenity::model::prelude::VoiceState;
use songbird::{
    create_player,
    driver::Bitrate,
    input::{
        self,
        cached::{Compressed, Memory},
        Input,
    },
    Call, Event, EventContext, EventHandler as VoiceEventHandler, SerenityInit, TrackEvent,
};

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    utils::MessageBuilder,
    Result as SerenityResult,
};
use strum::IntoEnumIterator;
use uuid::Uuid;

use ttsbot::tts;
use ttsbot::OptionStorage;
use ttsbot::{build_voice_text_options, build_voice_vox_options};

static LANGUAGE_DETECTOR: OnceCell<LanguageDetector> = OnceCell::new();
static TTS_CLIENT: OnceCell<tts::Client> = OnceCell::new();
// static OPTION_STORAGE: Lazy<RwLock<OptionStorage>> =
//     Lazy::new(|| RwLock::new(OptionStorage::new()));
static OPTION_STORAGE: OnceCell<RwLock<OptionStorage>> = OnceCell::new();
static BOT_JOINING_CHANNEL: OnceCell<RwLock<HashMap<GuildId, ChannelId>>> = OnceCell::new();

async fn play_voice(
    handler_lock: Arc<tokio::sync::Mutex<Call>>,
    text: impl fmt::Display,
    options: &tts::Options,
) -> anyhow::Result<()> {
    let detector = LANGUAGE_DETECTOR
        .get()
        .expect("Language detector is not initialized");
    if let Some(lang @ Language::Japanese) = detector.detect_language_of(text.to_string()) {
        let sound_src = {
            let sound_data = TTS_CLIENT
                .get()
                .expect("TTS_CLIENT is not initialized")
                .request(text, options)
                .await?;
            let temp_dir = env::temp_dir();
            // TODO: format
            let file_path = temp_dir.join(format!("ttsbot_{}.wav", Uuid::new_v4()));
            let mut file = File::create(&file_path)?;
            file.write_all(&sound_data)?;
            file.flush()?;
            Memory::new(input::ffmpeg(&file_path).await?)?
        };
        let _ = sound_src.raw.spawn_loader();
        let (mut audio, _) = create_player(sound_src.new_handle().try_into()?);
        audio.set_volume(0.1);
        let mut handler = handler_lock.lock().await;
        handler.play(audio);
    }
    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with(".") || msg.is_own(&ctx.cache).await {
            return;
        }

        let guild = msg.guild(&ctx.cache).await.unwrap();
        let guild_id = guild.id;

        {
            let authors_voice_channel_id = guild
                .voice_states
                .get(&msg.author.id)
                .and_then(|voice_state| voice_state.channel_id);

            let lock = BOT_JOINING_CHANNEL.get().unwrap().read();
            let bots_voice_channel_id = lock.get(&guild_id).cloned();

            if authors_voice_channel_id != bots_voice_channel_id {
                return;
            }
        }

        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let options = {
            let storage = OPTION_STORAGE.get().unwrap().read();
            storage.get(&msg.author.id)
        };

        if let Some(handler_lock) = manager.get(guild_id) {
            play_voice(handler_lock, msg.content_safe(&ctx.cache).await, &options)
                .await
                .ok();
        }
    }

    async fn voice_state_update(
        &self,
        ctx: Context,
        guild_id: Option<GuildId>,
        old_state: Option<VoiceState>,
        _: VoiceState,
    ) {
        if let Some(old_state) = old_state {
            let guild_id = guild_id.unwrap();
            let lock = BOT_JOINING_CHANNEL.get().unwrap().read();
            let bots_voice_channel_id = lock.get(&guild_id).cloned();
            if bots_voice_channel_id != old_state.channel_id {
                return;
            }

            if let Some(channel_id) = old_state.channel_id {
                let channel = ctx.cache.guild_channel(channel_id).await.unwrap();
                let members = channel.members(&ctx.cache).await.unwrap();
                if members.iter().filter(|m| !m.user.bot).count() == 0 {
                    let manager = songbird::get(&ctx)
                        .await
                        .expect("Songbird Voice client placed in at initialisation.")
                        .clone();
                    let has_handler = manager.get(guild_id).is_some();
                    if has_handler {
                        manager.remove(guild_id).await.unwrap();
                    }
                }
            }
        }
    }
}

#[group]
#[commands(engine, join, leave, mute, ping, preset, set, stop, unmute)]
struct General;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Opt {
    #[clap(long, env)]
    voicetext_api_key: String,

    #[clap(long, env)]
    voicevox_api_key: String,

    #[clap(long, env)]
    discord_token: String,

    #[clap(long, env)]
    database_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    dotenv().ok();
    let args = Opt::parse();

    TTS_CLIENT
        .set(tts::Client::new(
            args.voicetext_api_key,
            args.voicevox_api_key,
        ))
        .unwrap();

    LANGUAGE_DETECTOR
        .set(
            LanguageDetectorBuilder::from_languages(&[Language::English, Language::Japanese])
                .build(),
        )
        .ok();

    let storage = OptionStorage::connect(&args.database_url).await?;
    OPTION_STORAGE.set(RwLock::new(storage)).ok();

    BOT_JOINING_CHANNEL.set(RwLock::new(HashMap::new())).ok();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("."))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&args.discord_token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c().await?;
    println!("Received Ctrl-C, shutting down.");

    Ok(())
}

#[command]
async fn engine(context: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let print_usage = move || async {
        check_msg(
            msg.channel_id
                .say(
                    &context.http,
                    format!(
                        "`.engine {{{}}}`",
                        tts::Engine::iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<String>>()
                            .join("|")
                    ),
                )
                .await,
        );
    };

    if let Ok(engine) = args.single::<String>() {
        if let Ok(engine) = tts::Engine::try_from(engine.as_str()) {
            let content = match engine {
                tts::Engine::VoiceText => {
                    let api_url = "https://cloud.voicetext.jp/webapi/docs/api";
                    let speakers = tts::voice_text::VoiceTextSpeaker::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .join(", ");
                    format!("API: {api_url}\nAvailable speakers: {speakers}")
                }
                tts::Engine::VoiceVox => {
                    let official_url = "https://voicevox.hiroshiba.jp";
                    let api_url = "https://voicevox.su-shiki.com";
                    let speakers = tts::voice_vox::VoiceVoxSpeaker::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .join(", ");
                    format!(
                        "Official: {official_url}\nAPI: {api_url}\nAvailable speakers: {speakers}"
                    )
                }
            };
            check_msg(msg.channel_id.say(&context.http, content).await);
        } else {
            print_usage().await;
        }
    } else {
        print_usage().await;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let _handler = manager.join(guild_id, connect_to).await;

    let mut voice_channels = BOT_JOINING_CHANNEL.get().unwrap().write();
    voice_channels.insert(guild_id, connect_to);

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(msg.channel_id.say(&ctx.http, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Now muted").await);
    }

    Ok(())
}

#[command]
async fn ping(context: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.channel_id.say(&context.http, "Pong!").await);

    Ok(())
}

#[command]
async fn preset(context: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let print_usage = move || async {
        check_msg(
            msg.channel_id
                .say(
                    &context.http,
                    format!(
                        "Available presets: {}",
                        tts::Preset::iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                )
                .await,
        );
    };

    match args.single::<String>() {
        Ok(arg) => {
            if let Ok(preset) = tts::Preset::try_from(arg.as_str()) {
                {
                    let mut storage = OPTION_STORAGE.get().unwrap().write();
                    storage.set(&msg.author.id, tts::Options::from(preset)).await.unwrap();
                }

                let content = MessageBuilder::new()
                    .push("Set ")
                    .mention(&msg.author.id)
                    .push("'s preset: ")
                    .push(arg.as_str())
                    .build();
                check_msg(msg.channel_id.say(&context.http, content).await)
            } else {
                print_usage().await;
            }
        }
        Err(_) => {
            print_usage().await;
        }
    }

    Ok(())
}

#[command]
async fn set(context: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let print_usage = move || async {
        check_msg(
            msg.channel_id
                .say(
                    &context.http,
                    format!(
                        "`.set {{{}}} [key=value...]`",
                        tts::Engine::iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<String>>()
                            .join("|")
                    ),
                )
                .await,
        );
    };

    if let Ok(engine) = args.single::<String>() {
        if let Ok(engine) = tts::Engine::try_from(engine.as_str()) {
            match engine {
                tts::Engine::VoiceText => {
                    match build_voice_text_options(args.iter::<String>().map(|a| a.unwrap())) {
                        Ok(options) => {
                            let mut storage = OPTION_STORAGE.get().unwrap().write();
                            storage
                                .set(&msg.author.id, tts::Options::VoiceTextOptions(options))
                                .await?;
                        }
                        Err(e) => check_msg(msg.channel_id.say(&context.http, e.to_string()).await),
                    }
                }
                tts::Engine::VoiceVox => {
                    match build_voice_vox_options(args.iter::<String>().map(|a| a.unwrap())) {
                        Ok(options) => {
                            let mut storage = OPTION_STORAGE.get().unwrap().write();
                            storage
                                .set(&msg.author.id, tts::Options::VoiceVoxOptions(options))
                                .await?;
                        }
                        Err(e) => check_msg(msg.channel_id.say(&context.http, e.to_string()).await),
                    }
                }
            }
        } else {
            print_usage().await;
        }
    } else {
        print_usage().await;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);
            return Ok(());
        }
    };
    let mut handler = handler_lock.lock().await;
    handler.stop();
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Unmuted").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
