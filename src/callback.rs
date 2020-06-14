/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка нажатий на инлайн-кнопки. 14 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{CallbackQuery},
};

pub async fn handle_message(cx: DispatcherHandlerCx<CallbackQuery>) {
   let query = cx.update;
   let command = query.data;

   match cx.bot.answer_callback_query(query.id)
      .text("query.data.unwrap()")
      .send()
      .await {
      Err(_) => log::info!("error with handle_callback_query {}", command.unwrap_or_default()),
      _ => (),
   }
}