/* ===============================================================================
Restaurant menu bot.
User interface with inline buttons. 27 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*,
   types::{InputFile, InlineKeyboardButton, InlineKeyboardMarkup,
      CallbackQuery, ChatId, InputMedia, ParseMode, InputMediaPhoto,
      //ReplyMarkup, ButtonRequest, KeyboardButton,
   },
};
use strum::{AsRefStr, EnumString, };
use arraylib::iter::IteratorExt;

use crate::environment as env;
use crate::states::*;
use crate::database as db;
use crate::node::*;

#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "pas")]
   Pass(i32), // make the specified node active
   #[strum(to_string = "pan")]
   PassNow(i32), // like Pass but only among opened
   #[strum(to_string = "add")]
   Add(i32), // add node to basket
   // Remove, // remove node from basket
   Unknown,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Looking for the commands with arguments
      let cmd = s.get(..3).unwrap_or_default();
      let arg = |s: &str| {
         s.get(3..)
         .unwrap_or_default()
         .parse().unwrap_or_default()
      };

      if cmd == Self::Pass(0).as_ref() {
         let arg = arg;
         Command::Pass(arg(s))
      } else if cmd == Self::PassNow(0).as_ref() {
         let arg = arg;
         Command::PassNow(arg(s))
      } else if cmd == Self::Add(0).as_ref() {
         let arg = arg;
         Command::Add(arg(s))
      } else {
            Command::Unknown
      }
   }
}

pub async fn update(cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   let query = &cx.update;
   let query_id = &query.id;

   // Parse and process commands by receiving a message to send back
   let cmd = Command::parse(
      query.data.clone()
      .unwrap_or(String::default())
      .as_str()
   );
   let msg = match cmd {
      Command::Pass(id) => {
         view(id, WorkTime::All, &cx).await?;
         "Все заведения"
      }
      Command::PassNow(id) => {
         view(id, WorkTime::Now, &cx).await?;
         "Открытые сейчас"
      }
      Command::Add(_) => "В разработке",
      Command::Unknown => "Неизвестная команда",
   };

   // Отправляем ответ, который показывается во всплывающем окошке
   cx.requester.answer_callback_query(query_id)
   .text(msg)
   .send()
   .await
   .map_err(|err| format!("inline::update {}", err))?;

   Ok(())
}

pub async fn enter(state: CommandState, mode: WorkTime, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Load root node with children
   let load_mode = match mode {
      WorkTime::All => db::LoadNode::EnabledId(0),
      WorkTime::Now => db::LoadNode::EnabledNowId(0),
   };
   let node =  db::node(load_mode)
   .await
   .map_err(|s| map_req_err(s))?;

   if node.is_none() {
      cx.answer("Ошибка, нет записей - обратитесь к администратору")
      .await?;
   } else {

      let node = node.unwrap();

      // Next check - picture
      let picture = node.picture.clone();
      if picture.is_none() {
         cx.answer("Ошибка, без картинки невозможно продолжить - обратитесь к администратору")
         .await?;
      } else {

         // Notify about time
         let now = env::current_date_time();
         cx.answer(&format!("Заведения, открытые сейчас с учётом часового пояса ({}):", now.format("%H:%M")))
         .await?;

         // All is ok, collect and display info
         let picture = picture.unwrap();
         let markup = markup(&node, mode);
         let text = node_text(&node);

         cx.answer_photo(InputFile::file_id(picture))
         .caption(text)
         .reply_markup(markup)
         .parse_mode(ParseMode::Html)
         .disable_notification(true)
         .send()
         .await?;
      }
   }

   // Always stay in place
   next(state)
}

async fn msg(text: &str, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());
   cx.requester.send_message(chat_id, text)
   .await
   .map_err(|err| format!("inline::msg {}", err))?;
   Ok(())
}

async fn view(node_id: i32, mode: WorkTime, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {

   // Load node from database
   let load_mode = match mode {
      WorkTime::All => db::LoadNode::EnabledId(node_id),
      WorkTime::Now => db::LoadNode::EnabledNowId(node_id),
   };
   let node =  db::node(load_mode)
   .await?;
   if node.is_none() {
      msg("Ошибка, запись недействительна, начните заново", cx).await?;
      return Ok(())
   }

   // Collect info
   let node = node.unwrap();
   let markup = markup(&node, mode);
   let text = node_text(&node);

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());
   let message_id = message.id;

   // Приготовим структуру для редактирования
   let media = InputFile::file_id(node.picture.unwrap());
   let media = InputMediaPhoto::new(media)
   .caption(text)
   .parse_mode(ParseMode::Html);
   let media = InputMedia::Photo(media);

   // Отправляем изменения
   // cx.requester.send_photo(chat_id, media)
   // .caption(text)
   cx.requester.edit_message_media(chat_id, message_id, media)
   .reply_markup(markup)
   .await
   .map_err(|err| format!("inline::view {}", err))?;

   Ok(())
}

fn node_text(node: &Node) -> String {

   let mut res = format!("<b>{}</b>", node.title);

   // Do not display description from 1 symbol
   if node.descr.len() > 1 {
      res = res + "\n" + node.descr.as_str();
   };

   // Display time only if it sets
   if node.is_time_set() {
      res = format!("{}\nВремя работы: {}-{}",
         res,
         node.time.0.format("%H:%M"), node.time.1.format("%H:%M")
      )
   };

   if node.price != 0 {
      res = format!("{}\nЦена: {}",
         res,
         env::price_with_unit(node.price)
      )
   }

   res
}

fn markup(node: &Node, mode: WorkTime) -> InlineKeyboardMarkup {

   // Prepare command
   let pas = match mode {
      WorkTime::All => Command::Pass(0),
      WorkTime::Now => Command::PassNow(0),
   };
   let pas = String::from(pas.as_ref());

   // Create buttons for each child
   let buttons: Vec<InlineKeyboardButton> = node.children
   .iter()
   .map(|child| (InlineKeyboardButton::callback(
      child.title.clone(),
      format!("{}{}", pas, child.id)
   )))
   .collect();

   // Separate into long and short
   let (long, mut short) : (Vec<_>, Vec<_>) = buttons
   .into_iter()
   .partition(|n| n.text.chars().count() > 21);

   // If price not null add button for basket
   if node.price != 0 {
      let add = String::from(Command::Add(0).as_ref());
      let button_add = InlineKeyboardButton::callback(
         String::from("+🛒"),
         format!("{}{}", add, node.id)
      );
      short.push(button_add);
   }

   // Put in vec last unpaired button, if any
   let mut last_row = vec![];
   if short.len() % 2 == 1 {
      let unpaired = short.pop();
      if unpaired.is_some() {
         last_row.push(unpaired.unwrap());
      }
   }

   // Long buttons by one in row
   let markup = long.into_iter()
   .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));

   // Short by two
   let mut markup = short.into_iter().array_chunks::<[_; 2]>()
   .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));

   // Back button
   if node.id > 0 {
      let button_back = InlineKeyboardButton::callback(
         String::from("⏪Назад"),
         format!("{}{}", pas, node.parent)
      );
      last_row.push(button_back);
   }

   // Add the last unpaired button and the back button
   if !last_row.is_empty() {
      markup = markup.append_row(last_row);
   }

   markup
}
