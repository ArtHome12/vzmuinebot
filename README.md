# vzmuinebot

Telegram bot for food menu navigate

Disclaimer. The picture below is taken from the movie "Бриллиантовая рука" for illustration only https://ru.wikipedia.org/wiki/%D0%91%D1%80%D0%B8%D0%BB%D0%BB%D0%B8%D0%B0%D0%BD%D1%82%D0%BE%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D0%BA%D0%B0

### Start screen
![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme1s.jpg)

### After pressing the "Все" (means "All") button in the bottom menu
![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme2s.jpg)

 ### After pressing the inline button "Пример ресторана" (means "Restaurant example")
![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme3s.jpg)

### In the basket
![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme4s.jpg)

### Manager menu
![sheme](https://github.com/ArtHome12/vzmuinebot/blob/master/readme5s.jpg)

### Description
The person who deployed the bot is its administrator. Further:
* He advertises his bot as a platform for placing the menu
* restaurateurs turn to him
* he registers them in the system and restaurateurs manage their own menu



# Installation

Tested on hosting heroku.com, demo sample in telegram @Muine_vzbot - https://t.me/Muine_vzbot
To use the algorithm with another bot, you just need to specify a token.

At the first start, the algorithm creates the necessary tables on its own, but the database must already exist. The following environment variables must be set:

Connection to PostgeSQL database
`DATABASE_URL=postgres://ciiqzyjmfs...`

URL for webhook
`HOST=your_app_name.herokuapp.com`

Port of your https. Perhaps you should not set the port explicitly, it will provide the hosting (try first without this variable)
`PORT=443`

Token from bot father
`TELOXIDE_TOKEN=11344...`

For contact with you from caterers
`CONTACT_INFO=@none`

To identify you (up to three) as an admins - you can see your user_id when press button "Добавить"
`TELEGRAM_ADMIN_ID1=40966...`
`TELEGRAM_ADMIN_ID2=` can be blank
`TELEGRAM_ADMIN_ID3=`

To specify unit of price
`PRICE_UNIT=$`

To indicate the time zone
`TIME_ZONE=+7`

Optional. To specify service chat id - you can see it after add bot to group and send command /chat (/chat@yourbotname)
`LOG_GROUP_ID=-100123...`

# Service chat
The bot has the ability to send messages about some actions to a special service chat:
* new user logon
* ordering through a bot
* completion or cancellation of the order by the customer or manager

To enable this feature, you need to add the chat ID to the `LOG_GROUP_ID=ID` variable, as shown above. To find out the chat `ID`:
* add a bot to chat
* in chat send command `/chat` and bot will report the identifier.



The code is written using https://github.com/teloxide/teloxide and deployed with https://github.com/emk/heroku-buildpack-rust
Good luck!
