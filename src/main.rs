use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands, utils::markdown};

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
    let safe_message = markdown::escape(&message);
    return format!("```\n{safe_message}\n```");
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Ping => bot.send_message(msg.chat.id, "Pong").await?,
        Command::Languages => {
            let langs = godbolt::languages().await?;
            let message = format_languages(&langs);
            bot.send_message(msg.chat.id, message)
                .parse_mode(ParseMode::MarkdownV2)
                .await?
        }
    };

    Ok(())
}
