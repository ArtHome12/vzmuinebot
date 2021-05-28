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

use crate::states::*;
use crate::database as db;
use crate::node::*;

#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "pas")]
   Pass(i32), // make the specified node active
   Unknown,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Looking for the commands with arguments
      if s.get(..4).unwrap_or_default() == Self::Pass(0).as_ref() {
         let r_part = s.get(4..).unwrap_or_default();
         Command::Pass(r_part.parse().unwrap_or_default())
      } else {
         Command::Unknown
      }
   }
}


pub async fn update(cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   let query = &cx.update;
   let query_id = &query.id;


         // Код едока
         // let user_id = query.from.id;

   // Parse and process commands by receiving a message to send back
   let cmd = Command::parse(
      query.data.clone()
      .unwrap_or(String::default())
      .as_str()
   );
   let msg = match cmd {
      Command::Pass(id) => {"Success"}
      Command::Unknown => "Неизвестная команда"
   };

   // Отправляем ответ, который показывается во всплывающем окошке
   cx.requester.answer_callback_query(query_id)
   .text(msg)
   .send()
   .await
   .map_err(|err| format!("inline::update {}", err))?;

   Ok(())
}


pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Load root node with children
   let node =  db::node(db::LoadNode::Id(0))
   .await
   .and_then(|op| op.ok_or("Нет информации".into()))
   .map_err(|s| map_req_err(s))?;

   // Picture
   let picture = node.picture.clone();
   if picture.is_none() {
      cx.answer("Ошибка, без картинки невозможно продолжить - обратитесь к администратору")
      .await?;
   } else {
      let picture = picture.unwrap();
      let markup = markup(&node);
      
      cx.answer_photo(InputFile::file_id(picture))
      .caption("Hello")
      .reply_markup(markup)
      .disable_notification(true)
      .send()
      .await?;
   }

   // Always stay in place
   next(state)
}

async fn msg(text: &str, cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());
   cx.requester.send_message(chat_id, text)
   .await
   .map_err(|err| format!("inline::view {}", err))?;
   Ok(())
}

async fn view(node_id: i32, cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {

   // Load node from database
   let node =  db::node(db::LoadNode::Id(node_id))
   .await?;
   if node.is_none() {
      msg("Ошибка, запись недействительна, начните заново", cx).await?;
      return Ok(())
   }
   let node = node.unwrap();

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());
   let message_id = message.id;

   // Приготовим структуру для редактирования
   let media = InputMediaPhoto::new(InputFile::file_id(node.picture.unwrap()))
   .caption(node.title)
   .parse_mode(ParseMode::Html);
   let media = InputMedia::Photo(media);

   // Отправляем изменения
   cx.requester.edit_message_media(chat_id, message_id, media)
   // .reply_markup(data.markup)
   // .send()
   .await
   .map_err(|err| format!("inline::view {}", err))?;

   Ok(())
}

fn markup(node: &Node) -> InlineKeyboardMarkup {
   // Create buttons for each child
   let pas = String::from(Command::Pass(0).as_ref());
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
   let markup = short.into_iter().array_chunks::<[_; 2]>()
   .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));

   // Back button
   if node.id > 0 {
      let button_back = InlineKeyboardButton::callback(
         String::from("⏪Назад"), 
         format!("{}{}", pas, node.id)
      );
      last_row.push(button_back);
   }

   // Add the last unpaired button and the back button
   if last_row.is_empty() {
      markup
   } else {
      markup.append_row(last_row)
   }
}
