/* ===============================================================================
Restaurant menu bot.
Ticket ro placed order. 06 June 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*,
   types::{CallbackQuery, },
};


pub async fn make_ticket(_cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>, _node_id: i32) -> Result<&'static str, String> {
   Ok("В разработке!")
}