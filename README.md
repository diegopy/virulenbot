# virulenbot

The bot is configured via environment variables. See the .env.template file.

- `TELOXIDE_TOKEN`: Telegram Bot api token.
- `COINMARKETCAP_TOKEN`: Coin Market Cap API token.
- `RUST_LOG`: Log level
- `ENDPOINT`: If using webhook mode, which external URL routes to the bot. The bot will register this webhook at startup.
- `PORT`: Port to listen for webhook mode.
- `MODE`: Should be `webhook` for webhook mode, anything else for polling mode.
