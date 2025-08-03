# Telegram Compiler Explorer Bot

A Telegram bot that interacts with the [Compiler Explorer](https://godbolt.org/) API.

## Features

*   Compile code and get the assembly output.
*   Execute code and see the output.
*   List supported languages and compilers.

## Usage

The bot understands the following commands:

*   `/help` - Shows the help message.
*   `/ping` - Checks if the bot is alive.
*   `/languages` or `/ls` - Lists all supported languages.
*   `/compilers <language>` or `/c <language>` - Lists all supported compilers for a given language.
*   `/compile <compiler> <code>` - Compiles the given code with the specified compiler.
*   `/execute <code>` or `/e <code>` - Compiles and executes the given code.

## Building

1.  Clone the repository.
2.  Set the `TELOXIDE_TOKEN` environment variable to your Telegram bot token.
3.  Run `cargo run`.

## License

This project is licensed under the MIT License - see the [LICENSE.txt](LICENSE.txt) file for details.
