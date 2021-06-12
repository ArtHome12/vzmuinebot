/* ===============================================================================
Restaurant menu bot.
Callback from inline button. 27 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use strum::{AsRefStr, EnumString, EnumMessage, };

use teloxide::{
   prelude::*,
   types::{CallbackQuery, },
};
use crate::states::*;
use crate::database as db;
use crate::navigation;
use crate::registration;

#[derive(AsRefStr, EnumString, EnumMessage, )]
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
   TicketMake(i32), // start ordering through the bot
   #[strum(to_string = "tca", message = "Отмена заказа")]
   TicketCancel(i32), // cancel ticket
   #[strum(to_string = "tne", message = "Далее")]
   TicketNext(i32), // next stage for ticket
   #[strum(to_string = "tco", message = "Подтвердить")]
   TicketConfirm(i32), // finish ticket

   Unknown,
}

impl Command {
   pub fn parse(s: &str) -> Self {
      // Looking for the commands with arguments
      let cmd = s.get(..3).unwrap_or_default();
      let arg = s.get(3..)
      .unwrap_or_default()
      .parse().unwrap_or_default();

      if cmd == Self::Pass(0).as_ref() {
         Command::Pass(arg)
      } else if cmd == Self::PassNow(0).as_ref() {
         Command::PassNow(arg)
      } else if cmd == Self::IncAmount(0).as_ref() {
         Command::IncAmount(arg)
      } else if cmd == Self::IncAmountNow(0).as_ref() {
         Command::IncAmountNow(arg)
      } else if cmd == Self::DecAmount(0).as_ref() {
         Command::DecAmount(arg)
      } else if cmd == Self::DecAmountNow(0).as_ref() {
         Command::DecAmountNow(arg)
      } else if cmd == Self::TicketMake(0).as_ref() {
         Command::TicketMake(arg)
      } else if cmd == Self::TicketCancel(0).as_ref() {
         Command::TicketCancel(arg)
      } else if cmd == Self::TicketNext(0).as_ref() {
         Command::TicketNext(arg)
      } else if cmd == Self::TicketConfirm(0).as_ref() {
         Command::TicketConfirm(arg)
      } else {
         Command::Unknown
      }
   }
}

pub async fn update(cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), String> {
   async fn do_inc(node_id: i32, mode: WorkTime, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<&'static str, String> {
      // Increment amount in database and reload node
      let user_id = cx.update.from.id;
      db::orders_amount_inc(user_id, node_id).await?;
      navigation::view(node_id, mode, &cx).await?;
      Ok("Добавлено")
   }

   async fn do_dec(node_id: i32, mode: WorkTime, cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<&'static str, String> {
      // Decrement amount in database and reload node
      let user_id = cx.update.from.id;
      db::orders_amount_dec(user_id, node_id).await?;
      navigation::view(node_id, mode, &cx).await?;
      Ok("Удалено")
   }

   let query = &cx.update;
   let query_id = &query.id;

   // Parse and process commands by receiving a message to send back
   let cmd = Command::parse(
      query.data.clone()
      .unwrap_or(String::default())
      .as_str()
   );
   let msg = match cmd {
      Command::Pass(node_id) => {
         navigation::view(node_id, WorkTime::All, &cx).await?;
         "Все заведения"
      }
      Command::PassNow(node_id) => {
         navigation::view(node_id, WorkTime::Now, &cx).await?;
         "Открытые сейчас"
      }
      Command::IncAmount(node_id) => do_inc(node_id, WorkTime::All, &cx).await?,
      Command::IncAmountNow(node_id) => do_inc(node_id, WorkTime::Now, &cx).await?,
      Command::DecAmount(node_id) => do_dec(node_id, WorkTime::All, &cx).await?,
      Command::DecAmountNow(node_id) => do_dec(node_id, WorkTime::All, &cx).await?,
      Command::TicketMake(node_id) => registration::make_ticket(&cx, node_id).await?,
      Command::TicketCancel(node_id) => registration::cancel_ticket(&cx, node_id).await?,
      Command::TicketNext(node_id) => registration::next_ticket(&cx, node_id).await?,
      Command::TicketConfirm(node_id) => registration::confirm_ticket(&cx, node_id).await?,
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
