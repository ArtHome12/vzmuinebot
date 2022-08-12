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
use crate::loc::*;

pub async fn enter(bot: AutoSend<Bot>, msg: Message, state: MainState, mode: WorkTime) -> HandlerResult {

   let tag = state.tag;

   // Load root node with children
   let load_mode = match mode {
      WorkTime::All => db::LoadNode::EnabledId(0),
      WorkTime::Now => db::LoadNode::EnabledNowId(0),
      WorkTime::AllFrom(id) => db::LoadNode::EnabledId(id),
   };
   let node =  db::node(load_mode).await?;

   let chat_id = msg.chat.id;

   if node.is_none() {
      let text = match mode {
         WorkTime::Now => loc(Key::NavigationEnter1, tag, &[]), // "There is no currently open places"
         _ => loc(Key::NavigationEnter2, tag, &[]), // "Error, no entries - contact administrator"
      };

      bot.send_message(chat_id, text)
      .await?;
   } else {

      let node = node.unwrap();

      // Next check - picture
      match &node.picture {
         Origin::None => {
            // "Error, can't continue without picture - contact administrator"
            let text = loc(Key::NavigationEnter3, tag, &[]);
            bot.send_message(chat_id, text)
            .await?;
         }
         Origin::Own(picture) | Origin::Inherited(picture) => {

            // Notify about time
            if matches!(mode, WorkTime::Now) {
               let now = env::current_date_time();
               // "Establishments currently open, taking into account the time zone {} ({}):"
               let fmt = loc(Key::CommonTimeFormat, tag, &[]);
               let text = loc(Key::NavigationEnter4, tag, &[&env::time_zone_info(), &now.format(&fmt)]);
               bot.send_message(chat_id, text)
               .await?;
            }

            // All is ok, collect and display info
            let user_id = state.user_id; // user needs to sync with cart
            let markup = markup(&node, mode, user_id, tag).await?;
            let text = node_text(&node, tag);

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


async fn msg(bot: &AutoSend<Bot>, user_id: UserId, text: &str) -> Result<(), String> {
   bot.send_message(user_id, text)
   .await
   .map_err(|err| format!("inline::msg {}", err))?;
   Ok(())
}

pub async fn view(bot: &AutoSend<Bot>, q: CallbackQuery, node_id: i32, mode: WorkTime, tag: LocaleTag) -> Result<(), String> {

   let user_id = q.from.id;

   // Load node from database
   let load_mode = match mode {
      WorkTime::All | WorkTime::AllFrom(_) => db::LoadNode::EnabledId(node_id),
      WorkTime::Now => db::LoadNode::EnabledNowId(node_id),
   };
   let node =  db::node(load_mode).await?;
   if node.is_none() {
      // "Error, data deleted, start again"
      let text = loc(Key::NavigationView1, tag, &[]);
      msg(bot, user_id, &text).await?;
      return Ok(())
   }

   // Collect info
   let node = node.unwrap();
   let markup = markup(&node, mode, user_id, tag)
   .await?;

   let text = node_text(&node, tag);

   // Message to modify
   let message = q.message;
   if message.is_none() {
      // "Error, update message is invalid, please start again"
      let text = loc(Key::NavigationView2, tag, &[]);
      msg(bot, user_id, &text).await?;
      return Ok(())
   }
   // let chat_id = ChatId::Id(message.chat_id());
   let message_id = message.unwrap().id;

   // Picture is mandatory
   match node.picture {
      Origin::None => {
         // "Error, there is no picture, it is required - contact the staff"
         let text = loc(Key::NavigationView3, tag, &[]);
         msg(bot, user_id, &text).await?;
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

fn node_text(node: &Node, tag: LocaleTag) -> String {

   let mut res = format!("<b>{}</b>", node.title);

   // Do not display description from 1 symbol
   if node.descr.len() > 1 {
      res = res + "\n" + node.descr.as_str();
   };

   // Display time only if it sets
   if node.is_time_set() {
      // "{}\nWorking time: {}-{}"
      let fmt = loc(Key::CommonTimeFormat, tag, &[]);
      let args: Args = &[
         &res,
         &node.time.0.format(&fmt),
         &node.time.1.format(&fmt)
      ];
      res = loc(Key::NavigationNodeText1, tag, args);
   };

   if node.price != 0 {
      // "{}\nPrice: {}"
      res = loc(Key::NavigationNodeText2, tag, &[&res, &env::price_with_unit(node.price)])
   }

   res
}

async fn markup(node: &Node, mode: WorkTime, user_id: UserId, tag: LocaleTag) -> Result<InlineKeyboardMarkup, String> {

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

   // If price not null add button for cart with amount
   if node.price != 0 {
      // Display only title or title with amount
      let amount = db::orders_amount(user_id.0 as i64, node.id).await?;
      // "+🛒 ({})", "+🛒"
      let caption = if amount > 0 {
         loc(Key::NavigationMarkup1, tag, &[&amount])
      } else {
         loc(Key::NavigationMarkup2, tag, &[])
      };

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
            loc(Key::NavigationMarkup3, tag, &[]), // "-🛒"
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
         loc(Key::NavigationMarkup4, tag, &[]), // "⏪Back"
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
