# vzmuinebot
Telegram bot for food menu navigate 

Tested on hosting heroku.com, demo sample in telegram @Muine_vzbot
To use the algorithm with another bot, you just need to specify a token.

At the first start, the algorithm creates the necessary tables on its own, but the database must already exist. The following environment variables must be set:

Connection to PostgeSQL database
`DATABASE_URL=postgres://ciiqzyjmfs...`

URL for webhook
`HOST=vzmuinebot.herokuapp.com
PORT=443`

Token from bot father
`TELOXIDE_TOKEN=11344...`

For contact with you from caterers
`TELEGRAM_ADMIN_NAME=@none`

To identify you as an admin - you can see your user_id when press button "Добавить"
`TELEGRAM_ADMIN_ID=40966...`

Note. This is my first experience in learning the rust programming language and in bots, so the code is not very beautiful, it contains an excessive amount of copy-paste.
Good luck!
