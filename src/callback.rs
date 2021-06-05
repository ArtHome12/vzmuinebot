/* ===============================================================================
Restaurant menu bot.
Callback from inline button. 27 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use strum::{AsRefStr, EnumString, };

use teloxide::{
   prelude::*,
   types::{CallbackQuery, },
};
use crate::states::*;
use crate::database as db;
use crate::inline;

#[derive(AsRefStr, EnumString)]
pub enum Command {
   #[strum(to_string = "pas")]
   Pass(i32), // make the specified node active
   #[strum(to_string = "pan")]
   PassNow(i32), // like Pass but only among opened
   #[strum(to_string = "inc")]
   IncAmount(i32), // add 1pcs of node to basket and return to Pass mode
   #[strum(to_string = "inn")]
   IncAmountNow(i32), // add 1pcs of node to basket and return to PassNow mode
   #[strum(to_string = "dec")]
   DecAmount(i32), // remove 1pcs of node from basket and return to Pass mode
   #[strum(to_string = "den")]
   DecAmountNow(i32), // remove 1pcs of node from basket and return to PassNow mode
   #[strum(to_string = "tic")]
   MakeTicket(i32), // start ordering through the bot

   Unknown,
}

impl Command {
   pub fn parse(s: &str) -> Self {
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
      } else if cmd == Self::IncAmount(0).as_ref() {
         let arg = arg;
         Command::IncAmount(arg(s))
      } else if cmd == Self::IncAmountNow(0).as_ref() {
         let arg = arg;
         Command::IncAmountNow(arg(s))
      } else if cmd == Self::DecAmount(0).as_ref() {
         let arg = arg;
         Command::DecAmount(arg(s))
      } else if cmd == Self::DecAmountNow(0).as_ref() {
         let arg = arg;
         Command::DecAmountNow(arg(s))
      } else {
         Command::Unknown
      }
   }
}

pub async fn update(cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   async fn do_inc(node_id: i32, mode: WorkTime, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<&'static str, String> {
      // Increment amount in database and reload node
      let user_id = cx.update.from.id;
      db::amount_inc(user_id, node_id).await?;
      inline::view(node_id, mode, &cx).await?;
      Ok("Добавлено")
   }

   async fn do_dec(node_id: i32, mode: WorkTime, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<&'static str, String> {
      // Decrement amount in database and reload node
      let user_id = cx.update.from.id;
      db::amount_dec(user_id, node_id).await?;
      inline::view(node_id, mode, &cx).await?;
      Ok("Удалено")
   }

   let query = &cx.update;
   let query_id = &query.id;

   let debug = query.data.clone().unwrap_or(String::default());

   // Parse and process commands by receiving a message to send back
   let cmd = Command::parse(
      query.data.clone()
      .unwrap_or(String::default())
      .as_str()
   );
   let msg = match cmd {
      Command::Pass(node_id) => {
         inline::view(node_id, WorkTime::All, &cx).await?;
         "Все заведения"
      }
      Command::PassNow(node_id) => {
         inline::view(node_id, WorkTime::Now, &cx).await?;
         "Открытые сейчас"
      }
      Command::IncAmount(node_id) => do_inc(node_id, WorkTime::All, &cx).await?,
      Command::IncAmountNow(node_id) => do_inc(node_id, WorkTime::Now, &cx).await?,
      Command::DecAmount(node_id) => do_dec(node_id, WorkTime::All, &cx).await?,
      Command::DecAmountNow(node_id) => do_dec(node_id, WorkTime::All, &cx).await?,

      Command::MakeTicket(node_id) => {
         "В разработке"
      }

      // Command::Unknown => "Неизвестная команда",
      Command::Unknown => debug.as_str(),
   };

   // Отправляем ответ, который показывается во всплывающем окошке
   cx.requester.answer_callback_query(query_id)
   .text(msg)
   .send()
   .await
   .map_err(|err| format!("inline::update {}", err))?;

   Ok(())
}

