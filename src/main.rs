use std::borrow::Cow;
use teloxide::{
    payloads::SendMessage,
    prelude::*,
    requests::JsonRequest,
    types::{MessageEntityKind, ParseMode},
    utils::{command::BotCommands, markdown},
};

mod godbolt;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "Compiler Explorer (godbolt.org) bot. These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "pong.")]
    Ping,
    #[command(description = "compile the code from the message.", aliases = ["c"])]
    Compile,
    #[command(description = "list all supported languages.", aliases = ["ls"])]
    Languages,
    #[command(description = "list all supported compilers, specific language id can be specified.")]
    Compilers { language: String },
}

fn format_languages(langs: &[godbolt::Language]) -> String {
    let mut max_id_width = "id".len();
    let mut max_name_width = "name".len();

    for godbolt::Language { id, name } in langs {
        max_id_width = max_id_width.max(id.len());
        max_name_width = max_name_width.max(name.len());
    }

    let header = format!(
        "{:<id_width$} | {:<name_width$}",
        "id",
        "name",
        id_width = max_id_width,
        name_width = max_name_width
    );

    let separator = format!(
        "{:-<id_width$} | {:-<name_width$}",
        "",
        "",
        id_width = max_id_width,
        name_width = max_name_width
    );

    let formatted_langs = langs
        .iter()
        .map(|godbolt::Language { id, name }| {
            format!(
                "{:<id_width$} | {:<name_width$}",
                id,
                name,
                id_width = max_id_width,
                name_width = max_name_width
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let message = format!("{header}\n{separator}\n{formatted_langs}\n");
    return wrap_in_md(&message);
}

fn wrap_in_md(s: &str) -> String {
    let safe_s = markdown::escape(s);
    format!("```\n{safe_s}\n```")
}

fn trim_message(s: &str) -> Cow<str> {
    const TELEGRAM_MAX_MSG_LEN: usize = 4096;
    const TRUNCATION_SUFFIX_PLAIN: &str = "\n... (message trimmed)";
    const TRUNCATION_SUFFIX_MD: &str = "\n... (message trimmed)```";
    if s.chars().count() <= TELEGRAM_MAX_MSG_LEN {
        return Cow::Borrowed(s);
    }

    let suffix = if s.starts_with("```") && s.ends_with("```") {
        TRUNCATION_SUFFIX_MD
    } else {
        TRUNCATION_SUFFIX_PLAIN
    };

    let suffix_len = suffix.chars().count();
    let max_len = TELEGRAM_MAX_MSG_LEN - suffix_len;

    let end_index = s
        .char_indices()
        .nth(max_len)
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    let truncated_s = &s[..end_index];

    Cow::Owned(format!("{}{}", truncated_s, suffix))
}

fn send_message(bot: &Bot, chat_id: ChatId, s: &str) -> JsonRequest<SendMessage> {
    bot.send_message(chat_id, trim_message(s))
}

async fn send_md(bot: &Bot, chat_id: ChatId, s: &str) -> ResponseResult<()> {
    send_message(bot, chat_id, s)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
}

fn format_compilers(compilers: &[&godbolt::Compiler]) -> String {
    if compilers.is_empty() {
        return "No compilers found for this language.".to_string();
    }

    let mut max_id_width = "ID".len();
    let mut max_name_width = "Name".len();
    let mut max_version_width = "Version".len();

    for compiler in compilers {
        max_id_width = max_id_width.max(compiler.id.len());
        max_name_width = max_name_width.max(compiler.name.len());
        max_version_width = max_version_width.max(compiler.semver.len());
    }

    let header = format!(
        "{:<id_w$} | {:<name_w$} | {:<ver_w$}",
        "ID",
        "Name",
        "Version",
        id_w = max_id_width,
        name_w = max_name_width,
        ver_w = max_version_width
    );

    let separator = format!(
        "{:-<id_w$} | {:-<name_w$} | {:-<ver_w$}",
        "",
        "",
        "",
        id_w = max_id_width,
        name_w = max_name_width,
        ver_w = max_version_width
    );

    let rows: Vec<String> = compilers
        .iter()
        .map(|compiler| {
            format!(
                "{:<id_w$} | {:<name_w$} | {:<ver_w$}",
                compiler.id,
                compiler.name,
                compiler.semver,
                id_w = max_id_width,
                name_w = max_name_width,
                ver_w = max_version_width
            )
        })
        .collect();

    let mut output_lines = Vec::with_capacity(2 + rows.len());
    output_lines.push(header);
    output_lines.push(separator);
    output_lines.extend(rows);

    wrap_in_md(&output_lines.join("\n"))
}

fn parse_compilers_language(s: &str) -> (&str, &str) {
    let s = s.trim();

    match s.split_once(char::is_whitespace) {
        Some((first, remainder)) => (first, remainder),
        None => {
            if s.is_empty() {
                ("", "")
            } else {
                (s, "")
            }
        }
    }
}

fn parse_compile_msg(msg: &Message) -> Result<(String, String), String> {
    let parsed_entities = msg.parse_entities().unwrap_or_default();
    let code_block = parsed_entities
        .iter()
        .filter_map(|entity| match entity.kind() {
            MessageEntityKind::Pre { .. } | MessageEntityKind::Code => {
                Some(entity.text().to_string())
            }
            _ => None,
        })
        .collect::<Vec<String>>();
    let code_block_len = code_block.len();
    if code_block_len != 1 {
        let error_text =
            format!("Invalid format. Expected exactly one code block, got {code_block_len}.");
        return Err(error_text);
    }
    let code = code_block.first().unwrap();
    let text = msg.text().unwrap_or_default();
    let compile_full_command = text.replace(code, "");
    let compiler_commands = compile_full_command
        .split_whitespace()
        .collect::<Vec<&str>>();
    let compiler_id = match compiler_commands[..] {
        [_command, compiler_id, ..] => Some(compiler_id),
        [..] => None,
    };

    if let Some(id) = compiler_id {
        return Ok((id.to_string(), code.to_string()));
    } else {
        return Err("Invalid format. Expected compile command.".to_string());
    }
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Ping => {
            bot.send_message(msg.chat.id, "Pong").await?;
        }
        Command::Languages => {
            let langs = godbolt::languages().await?;
            let message = format_languages(&langs);
            send_md(&bot, msg.chat.id, &message).await?;
        }
        Command::Compile => {
            let parse_result = parse_compile_msg(&msg);
            match parse_result {
                Ok((id, code)) => {
                    let res = godbolt::compile(&id, &code).await?;
                    match res {
                        godbolt::CompilationOutput::Assembly(assembly) => {
                            log::info!("Assembly: {assembly}");
                            send_md(&bot, msg.chat.id, &wrap_in_md(&assembly)).await?;
                        }
                        godbolt::CompilationOutput::Stderr(raw_err) => {
                            log::info!("Error: {raw_err}");
                            let err = strip_ansi_escapes::strip_str(&raw_err);
                            send_md(&bot, msg.chat.id, &wrap_in_md(&err)).await?;
                        }
                    }
                }
                Err(error) => {
                    send_message(&bot, msg.chat.id, &error).await?;
                }
            }
        }
        Command::Compilers { language } => {
            let (language_id, filter) = parse_compilers_language(&language);
            let compilers = godbolt::compilers_for_language(language_id).await?;
            let filtered_compilers = compilers
                .iter()
                .filter(|compiler| compiler.name.contains(filter))
                .collect::<Vec<&godbolt::Compiler>>();
            let message = format_compilers(&filtered_compilers);
            send_md(&bot, msg.chat.id, &message).await?;
        }
    };

    Ok(())
}
