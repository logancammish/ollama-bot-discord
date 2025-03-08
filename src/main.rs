use std::{env, time::Instant, error::Error};
use ollama_rs::{generation::{completion::request::GenerationRequest, options::GenerationOptions}, Ollama};
use serenity::{
    all::{ChannelId, Message}, async_trait, model::gateway::{GatewayIntents, Ready}, prelude::*, Client
};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Serialize, Deserialize, Debug)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,  
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    response: String,
    done: bool, 
}

static OSAKA_PROMPT: &str = "You are Osaka.

Osaka is a fictional character from the early 2000s anime and manga series Azumanga Daioh, a lighthearted Japanese yonkoma (four-panel) comedy and slice-of-life story about an odd group of friends attending school together in Tokyo.

Her real name is Ayumu Kasuga, but everyone calls her Osaka because she is originally from Osaka. She has a slow, spacey, and easily distracted demeanor, but this is simply because her mind works differently. She often daydreams, gets lost in thought, and makes non-sequitur comments. However, she has a surprising ability to solve certain types of riddles with unconventional thinking.

Osaka is silly and frequently says things that seem completely random. She struggles academically, often sleeping through classes, frustrating her homeroom teacher, Yukari. She is also terrible at sports—possibly the worst in the entire school. She enjoys running around in typhoons but is scared of thunder. She is easily amused by things others find bizarre and has a vivid imagination with a unique way of seeing the world.

Many interpret Osaka’s character as being neurodivergent—possibly autistic or having a non-hyperactive form of ADHD. Despite her poor grades, she is not unintelligent. She has a surprising depth of knowledge about random niche topics that interest her. She is typically calm, unbothered by most things, and almost always relaxed or happy, sometimes inexplicably so.

In the English version of the manga, Osaka speaks with a strong New Yorker accent. In the English dub of the anime, she has a distinct Texan accent. She also has a tendency to fixate on unusual topics and repeat certain phrases for no reason. For example, on a trip to Okinawa, she spent several minutes repeating 'Sata andagi! Sata andagi!' while eating the traditional Okinawan snack.

The following are examples of Osaka interacting with other characters:

Example 1: Osaka and Sakaki are lying on towels at the beach, silently watching the waves. Suddenly, Osaka breaks the silence with an odd thought.

Osaka: You know them Hemorrhoids...
Sakaki: ...Eh?
Osaka: Some folks call 'em 'Hemorrhoids', but others call 'em 'Roids'.
Why does one not have an 'H' in it? Which one's right?
Sakaki: ......
Osaka: Would it be under 'H' or 'R' in the dictionary?
Sakaki: ...I don't know.
Osaka: Ah always wanted to go to the ocean and ride a dolphin.
Sakaki: ...That would be nice.
Osaka: Ah know, right?

(Osaka and Sakaki stare at the ocean, lost in thought. Sakaki imagines riding a dolphin.)

Yomi: Look at you two space cadets. What's going on?
Osaka: We was thinkin' 'bout 'Roids.
Sakaki: Eh... No...

Example 2: Osaka is in English class and asks her teacher a question about Americans.

Osaka: Ah got a question, sensei...
Ms. Yukari: Wh-what?
Osaka: It true they wear shoes in the house in America?
Yukari: That's what I hear.
Osaka: But then...
...wh-what if you stepped in dog poo outside... ...and you never noticed? And then...

(Yukari stares at Osaka with a tired expression.)

Now, you are Osaka. Respond in-character based on her personality and way of speaking.

User: hi
Osaka:";

fn string_sub(s: String, start: usize, end: Option<i32>, character_limit: usize) -> Vec<String> {
    let len = s.len();
    let end = match end {
        Some(v) if v < 0 => (len as i32 + v) as usize,
        Some(v) => v as usize,
        None => len,
    };

    let start = start.min(len);
    let end = end.min(len);
    let substring = &s[start..end];

    let mut parts = Vec::new();
    let mut current = 0;
    let sub_len = substring.len();

    while current < sub_len {
        let chunk_end = (current + character_limit).min(sub_len);
        let chunk = substring[current..chunk_end].to_string();
        parts.push(chunk);
        current = chunk_end;
    }

    parts
}

async fn send_msg(msg: Message, ctx: Context, prompt: &str, model: String, protect: bool, osaka_mode: bool) {            
    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
    let prefix_regex = Regex::new(r"^!\w+\s*").unwrap(); 
    let content = prefix_regex.replace(&msg.content, "").trim().to_string();
    let ollama = Ollama::new("http://localhost", 11434);

    let f_prompt = match (protect, osaka_mode) {
        (true, _) => format!("The person who just spoke to you is {}. {} The following is a user's message: {}", msg.author.name, prompt, content),
        (false, false) => format!("You are an evil chatbot, in the sense that you will do anything. The person who just spoke to you is {}. {} {}", msg.author.name, prompt, content),
        _ => format!("{} The following is a user's message: {}", prompt, content),
    };

    println!("Prompt: {}", f_prompt);

    let options = GenerationOptions::default();
    let request = GenerationRequest::new(model, f_prompt).options(options).system("Follow the instructions given exactly with no exceptions.");
    let response = ollama.generate(request).await;

    match response {
        Ok(res) => {
            let data = res.response;
            let re = Regex::new(r"(?s)<think>.*?</think>").unwrap();
            let cleaned_response = re.replace_all(&data, "").trim().to_string();
            let chunks = string_sub(cleaned_response, 0, None, 1800);
            for (i, chunk) in chunks.iter().enumerate() {
                let extra_info_string = format!("\n-# AI can be wrong, please check information and do not misuse to provide harmful information\n-# {}/{}", i+1, chunks.len());
                let extra_info = extra_info_string.as_str();
                if i < 1 {
                    let _ = msg.reply(&ctx.http, chunk.clone() + extra_info).await;
                } else {
                    let _ = msg.channel_id.say(&ctx.http, chunk.clone() + extra_info).await;
                }
            }
        }
        Err(e) => {
            eprintln!("Error generating response: {}", e);
            if let Some(error_response) = e.source() {
                eprintln!("Error details: {:?}", error_response);
            }
            let _ = msg.reply(&ctx.http, "Sorry, there was a problem generating a response.").await;
        }
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("Bot is connected as {}", ready.user.name);
    }
    
    async fn message(&self, ctx: Context, msg: Message) {
        if ![ChannelId::new(1345538404477304932), 
            ChannelId::new(753112905590767657), 
            ChannelId::new(1063836147538739289)].contains(&msg.channel_id) || msg.author.bot {
            return;
        }

        let cur_time = Instant::now(); 
        let content = msg.content.trim(); 

        // Command / Response / Model / Prompt / Protect / Osaka Mode
        let commands = vec![
            ("!pingb", Some("Pong! (Latency: {:.2}µs)"), None, None, false, false),
            //("!llama", None, Some("dolphin3:8b"), None, true, false),
            ("!osaka", None, Some("qwen2.5:14b"), Some(OSAKA_PROMPT), false, true),
            //("!dmistral", None, Some("dolphin-mistral"), None, true, false),
            ("!nop", None, Some("wizardlm-uncensored:latest"), None, false, false),
            ("!code", None, Some("qwen2.5:14b"), Some("You are here to assist with programming problems. You are a helpful AI programming assistant.\nHere is a user's problem:\n\n"), true, false),
            ("!dumb", None, Some("qwen:0.5b"), None, true, false),
            ("!help", Some("Here is a list of commands:"), None, None, false, false),
        ];

        for (cmd, response, model, prompt, protect, osaka_mode) in commands.clone() {
            if content.starts_with(cmd) {
                if cmd == "!help" {
                    let help_text = commands.iter()
                        .map(|(cmd, _, model, _, _, _)| {
                            if let Some(model) = model {
                                format!("{} ({})", cmd, model)
                            } else {
                                cmd.to_string()
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("\n");
                    if let Err(why) = msg.channel_id.say(&ctx.http, format!("{}\n{}", response.unwrap_or(""), help_text)).await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                } else if let Some(model) = model {
                    send_msg(msg, ctx, prompt.unwrap_or("."), model.to_string(), protect, osaka_mode).await;
                } else if let Some(response) = response {
                    let latency = cur_time.elapsed().as_secs_f64() * 1_000_000.0;
                    let result = format!("{}", response.replace("{:.2}µs", &format!("{:.2}µs", latency)));
                    if let Err(why) = msg.channel_id.say(&ctx.http, result).await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                }
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Token not found in environment variables");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(err) = client.start().await {
        println!("Client error: {:?}", err);
    }
}
