use teloxide::{
    prelude::*, types::MessageEntityKind, types::ParseMode, utils::command::BotCommands,
    utils::markdown,
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

async fn send_md(bot: &Bot, chat_id: ChatId, s: &str) -> ResponseResult<()> {
    bot.send_message(chat_id, s)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
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
                            bot.send_message(msg.chat.id, wrap_in_md(&assembly))
                                .parse_mode(ParseMode::MarkdownV2)
                                .await?;
                        }
                        godbolt::CompilationOutput::Stderr(raw_err) => {
                            log::info!("Error: {raw_err}");
                            let err = strip_ansi_escapes::strip_str(&raw_err);
                            bot.send_message(msg.chat.id, wrap_in_md(&err))
                                .parse_mode(ParseMode::MarkdownV2)
                                .await?;
                        }
                    }
                }
                Err(error) => {
                    bot.send_message(msg.chat.id, error).await?;
                }
            }
        }
    };

    Ok(())
}
