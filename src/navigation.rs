/* ===============================================================================
Restaurant menu bot.
User interface with inline buttons. 27 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*,
   types::{InputFile, InlineKeyboardButton, InlineKeyboardMarkup,
      CallbackQuery, InputMedia, ParseMode, InputMediaPhoto,
   },
};
use arraylib::iter::IteratorExt;
use crate::callback::Command;

use crate::environment as env;
use crate::states::*;
use crate::database as db;
use crate::node::*;

pub async fn enter(bot: AutoSend<Bot>, msg: Message, state: MainState, mode: WorkTime) -> HandlerResult {

   // Load root node with children
   let load_mode = match mode {
      WorkTime::All => db::LoadNode::EnabledId(0),
      WorkTime::Now => db::LoadNode::EnabledNowId(0),
      WorkTime::AllFrom(id) => db::LoadNode::EnabledId(id),
   };
   let node =  db::node(load_mode).await?;

   let chat_id = msg.chat.id;

   if node.is_none() {
      bot.send_message(chat_id, "Ошибка, нет записей - обратитесь к администратору")
      .await?;
   } else {

      let node = node.unwrap();

      // Next check - picture
      match &node.picture {
         Origin::None => {
            bot.send_message(chat_id, "Ошибка, без картинки невозможно продолжить - обратитесь к администратору")
            .await?;
         }
         Origin::Own(picture) | Origin::Inherited(picture) => {

            // Notify about time
            if matches!(mode, WorkTime::Now) {
               let now = env::current_date_time();
               bot.send_message(chat_id, &format!("Заведения, открытые сейчас с учётом часового пояса {} ({}):", env::time_zone_info(), now.format("%H:%M")))
               .await?;
            }

            // All is ok, collect and display info
            let user_id = state.user_id; // user needs to sync with basket
            let markup = markup(&node, mode, user_id)
            .await
            .map_err(|s| map_req_err(s))?;
            let text = node_text(&node);

            bot.send_photo(chat_id, InputFile::file_id(picture))
            .caption(text)
            .reply_markup(markup)
            .parse_mode(ParseMode::Html)
            .disable_notification(true)
            .await?;
         }
      }
   }

   Ok(())
}


async fn msg(bot: AutoSend<Bot>, user_id: UserId, text: &str) -> Result<(), String> {
   bot.send_message(user_id, text)
   .await
   .map_err(|err| format!("inline::msg {}", err))?;
   Ok(())
}

pub async fn view(bot: AutoSend<Bot>, q: CallbackQuery, node_id: i32, mode: WorkTime) -> Result<(), String> {

   let user_id = q.from.id;

   // Load node from database
   let load_mode = match mode {
      WorkTime::All | WorkTime::AllFrom(_) => db::LoadNode::EnabledId(node_id),
      WorkTime::Now => db::LoadNode::EnabledNowId(node_id),
   };
   let node =  db::node(load_mode).await?;
   if node.is_none() {
      msg(bot, user_id, "Ошибка, запись недействительна, начните заново").await?;
      return Ok(())
   }

   // Collect info
   let node = node.unwrap();
   let markup = markup(&node, mode, user_id)
   .await?;

   let text = node_text(&node);

   // Message to modify
   let message = q.message;
   if message.is_none() {
      msg(bot, user_id, "Ошибка, сообщение недействительно, начните заново").await?;
      return Ok(())
   }
   // let chat_id = ChatId::Id(message.chat_id());
   let message_id = message.unwrap().id;

   // Picture is mandatory
   match node.picture {
      Origin::None => {
         msg(bot, user_id, "Ошибка, отсутствует картинка, она обязательна").await?;
         return Ok(())
      }
      Origin::Own(picture_id) | Origin::Inherited(picture_id) => {

         // Prepare data to edit
         let media = InputFile::file_id(picture_id);
         let media = InputMediaPhoto::new(media)
         .caption(text)
         .parse_mode(ParseMode::Html);
         let media = InputMedia::Photo(media);

         // Send data
         bot.edit_message_media(user_id, message_id, media)
         .reply_markup(markup)
         .await
         .map_err(|err| format!("inline::view {}", err))?;

         Ok(())
      }
   }
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

async fn markup(node: &Node, mode: WorkTime, user_id: UserId) -> Result<InlineKeyboardMarkup, String> {

   // Prepare command
   let pas = match mode {
      WorkTime::All | WorkTime::AllFrom(_) => Command::Pass(0),
      WorkTime::Now => Command::PassNow(0),
   };
   let pas = String::from(pas.as_ref());

   // Create buttons for each child
   let buttons: Vec<InlineKeyboardButton> = node.children
   .iter()
   .map(|child| {
      InlineKeyboardButton::callback(
      child.title_with_price(),
      format!("{}{}", pas, child.id)
   )})
   .collect();

   // Separate into long and short
   let (long, mut short) : (Vec<_>, Vec<_>) = buttons
   .into_iter()
   .partition(|n| n.text.chars().count() > 21);

   // If price not null add button for basket with amount
   if node.price != 0 {
      // Display only title or title with amount
      let amount = db::orders_amount(user_id.0 as i64, node.id).await?;
      let caption = if amount > 0 { format!("+🛒 ({})", amount) } else { String::from("+🛒") };

      let cmd = match mode {
         WorkTime::All | WorkTime::AllFrom(_) => Command::IncAmount(0),
         WorkTime::Now => Command::IncAmountNow(0),
      };
      let cmd = String::from(cmd.as_ref());
      let button_inc = InlineKeyboardButton::callback(
         caption,
         format!("{}{}", cmd, node.id)
      );
      short.push(button_inc);

      // Add decrease button
      if amount > 0 {
         let cmd = match mode {
            WorkTime::All | WorkTime::AllFrom(_) => Command::DecAmount(0),
            WorkTime::Now => Command::DecAmountNow(0),
         };
         let cmd = String::from(cmd.as_ref());
         let button_dec = InlineKeyboardButton::callback(
            String::from("-🛒"),
            format!("{}{}", cmd, node.id)
         );
         short.push(button_dec);
      }
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

   Ok(markup)
}
