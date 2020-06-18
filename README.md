# vzmuinebot
Telegram bot for food menu navigate 

Disclaimer. The picture below is taken from the movie "Бриллиантовая рука" for illustration only https://ru.wikipedia.org/wiki/%D0%91%D1%80%D0%B8%D0%BB%D0%BB%D0%B8%D0%B0%D0%BD%D1%82%D0%BE%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D0%BA%D0%B0

![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme.png)

The person who deployed the bot is its administrator. Further:
* He advertises his bot as a platform for placing the menu
* restaurateurs turn to him
* he registers them in the system and restaurateurs manage their own menu

# Installation
Tested on hosting heroku.com, demo sample in telegram @Muine_vzbot
To use the algorithm with another bot, you just need to specify a token.

At the first start, the algorithm creates the necessary tables on its own, but the database must already exist. The following environment variables must be set:

Connection to PostgeSQL database
`DATABASE_URL=postgres://ciiqzyjmfs...`

URL for webhook
`HOST=vzmuinebot.herokuapp.com`

Port of your https
`PORT=443`

Token from bot father
`TELOXIDE_TOKEN=11344...`

For contact with you from caterers
`TELEGRAM_ADMIN_NAME=@none`

To identify you as an admin - you can see your user_id when press button "Добавить"
`TELEGRAM_ADMIN_ID=40966...`

To specify unit of price
`PRICE_UNIT=$`

To indicate the time zone
`TIME_ZONE=+7`


Optional. To specify service chat id - you can see it after add bot to group and send command /chat (/chat@yourbotname)
`LOG_GROUP_ID=-100123...`

# Commands
This commands should be entered only in the main (first) menu.
* To register (or enable) a new restaurant, enter the command `/regi12345...`, where 12345 is user id of new caterer.
* To disable restaurant `/hold12345...` 
* To enter as owner some restaurant `/sudo123`, where 123 is the serial number (not user_id!) of the restaurant.
* To see the list of restaurants `/list`
* To see id of current chat `/chat`. Bot shows your Id if you in private chat with bot or group id (negative number)

This commands should be entered only in the caterer (where editing restaraunt title, info etc.) menu.
* Transfer ownership of the restaurant to another user `/move12345...`, where 12345 is user id of new caterer.

Note. This is my first experience in learning the rust programming language and in bots, so the code is not very beautiful, it contains an excessive amount of copy-paste.
The code is written using https://github.com/teloxide/teloxide and deployed with https://github.com/emk/heroku-buildpack-rust
Good luck!
