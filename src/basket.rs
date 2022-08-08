/* ===============================================================================
Restaurant menu bot.
Basket menu. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, 
      ParseMode, ButtonRequest, InlineKeyboardButton, InlineKeyboardMarkup,
   }
};

use std::str::FromStr;
use strum::{AsRefStr };

use crate::states::*;
use crate::database as db;
use crate::customer::*;
use crate::environment as env;
use crate::callback as cb;
use crate::node;
use crate::orders;
use crate::registration;
use crate::general;
use crate::loc::{loc, Key, LocaleTag};


// ============================================================================
// [Main entry]
// ============================================================================

// Main commands
#[derive(Copy, Clone)]
enum Command {
   Clear, // add a new node
   Exit, // return to start menu
   Edit(EditCmd),
   Delete(i32),
   Reload,
   Unknown,
}

// Main commands
#[derive(Copy, Clone, AsRefStr)]
enum EditCmd {
   #[strum(to_string = "name")] // Button caption and db field name
   Name,
   #[strum(to_string = "contact")]
   Contact,
   #[strum(to_string = "address")]
   Address,
   #[strum(to_string = "delivery")]
   Delivery,
}

impl Command {
   fn parse(s: &str, tag: LocaleTag) -> Self {
      // Try as command without arguments
      if s == loc::<char>(Key::BasketCommandClear, tag, &[]) { Self::Clear }
      else if s == loc::<char>(Key::BasketCommandExit, tag, &[]) { Self::Exit }
      else if s == loc::<char>(Key::BasketCommandReload, tag, &[]) { Self::Reload }
      else if s == loc::<char>(Key::BasketCommandEditName, tag, &[]) { Self::Edit(EditCmd::Name) }
      else if s == loc::<char>(Key::BasketCommandEditContact, tag, &[]) { Self::Edit(EditCmd::Contact) }
      else if s == loc::<char>(Key::BasketCommandEditAddress, tag, &[]) { Self::Edit(EditCmd::Address) }
      else if s == loc::<char>(Key::BasketCommandEditDelivery, tag, &[]) { Self::Edit(EditCmd::Delivery) }
      else {
         // Looking for the commands with arguments
         if s.get(..4).unwrap_or_default() ==  loc::<char>(Key::BasketCommandDelete, tag, &[]) {
            let r_part = s.get(4..).unwrap_or_default();
            Self::Delete(r_part.parse().unwrap_or_default())
         } else {
            Self::Unknown
         }
      }
   }
}

#[derive(Clone)]
pub struct BasketState {
   pub prev_state: MainState,
   pub customer: Customer,
}

pub async fn enter(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {

   // Load user info
   let customer = db::user(state.user_id.0).await?;

   // Display
   let state = BasketState { prev_state: state, customer };
   dialogue.update(state.to_owned()).await?;
   view(bot, msg, state).await
}


async fn view(bot: AutoSend<Bot>, msg: Message, state: BasketState) -> HandlerResult {
   let tag = state.prev_state.locale;

   // Start with info about user
   // "Your data, {}:\nContact for communication: {}\nDelivery method: {}"
   let args = [&state.customer.name, &state.customer.contact, &state.customer.delivery_desc()];
   let info = loc(Key::BasketView1, tag, &args);

   // Load info about orders
   let user_id = state.prev_state.user_id;
   let orders = db::orders(user_id.0 as i64).await?;

   // Announce
   let basket_info = orders.basket_info();
   let announce = if basket_info.orders_num == 0 {
      // "Cart is empty"
      loc::<char>(Key::BasketView2, tag, &[])
   } else {
      // "In cart {} pos., {} pcs. for total cost {}"
      let args = [&basket_info.orders_num.to_string(),
         &basket_info.items_num.to_string(),
         &env::price_with_unit(basket_info.total_cost)
      ];
      loc(Key::BasketView3, tag, &args)
   };
   bot.send_message(msg.chat.id, format!("{}\n\n{}", info, announce))
   .reply_markup(markup(tag))
   .await?;

   // Messages by owners
   for owner in orders.data {
      let owner_id = owner.0.id;
      let text = make_owner_text(&owner.0, &owner.1, tag);

      
      bot.send_message(msg.chat.id, text)
      .reply_markup(order_markup(owner_id, tag))
      .parse_mode(ParseMode::Html)
      .await?;
   }

   // Show tickets (orders in process)
   registration::show_tickets(bot, state.prev_state.user_id).await?;

   Ok(())
}

pub async fn update(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: BasketState) -> HandlerResult {

   let tag = state.prev_state.locale;
   let user_id = state.prev_state.user_id.0 as u64;

   // Parse and handle commands
   let text = msg.text().unwrap_or_default();
   let cmd = Command::parse(text, tag);
   match cmd {
      Command::Clear => {
         // Remove all orders from database and update user screen
         db::orders_delete(user_id).await?;
         view(bot, msg, state).await
      }

      Command::Exit => crate::states::reload(bot, msg, dialogue, state.prev_state).await,

      Command::Edit(cmd) => {
         let new_state = BasketStateEditing { prev_state: state, cmd };
         dialogue.update(new_state.to_owned()).await?;
         enter_edit(bot, msg, new_state).await
      }

      Command::Delete(node_id) => {
         // Remove the order from database and update user screen
         db::order_delete_node(user_id, node_id).await?;
         view(bot, msg, state).await
      }

      Command::Reload => view(bot, msg, state).await,

      Command::Unknown => {
         // "You are leaving the order menu"
         let text = loc::<char>(Key::BasketUpdate, tag, &[]);
         bot.send_message(msg.chat.id, text).await?;

         // General commands handler - messaging, searching...
         dialogue.update(state.prev_state).await?;
         general::update(bot, msg, dialogue, state.prev_state).await
      }
   }
}

pub fn make_owner_text(node: &node::Node, order: &orders::Order, tag: LocaleTag) -> String {

   // Prepare info about owner
   let descr = if node.descr.len() <= 1 { String::default() } 
   else { format!("\n{}", node.descr) };

   let time = if node.time.0 == node.time.1 {
      // "\nOpening hours: around the clock"
      loc::<char>(Key::BasketMakeOwnerText1, tag, &[])
   } else {
      // "\nOpening hours: {}-{}"
      let open_time = loc(Key::BasketMakeOwnerText3, tag, &[&node.time.0]);
      let close_time = loc(Key::BasketMakeOwnerText3, tag, &[&node.time.1]);
      loc(Key::BasketMakeOwnerText2, tag, &[&open_time, &close_time])
   };

   // Info about items
   let items = order.iter()
   .fold(String::from("\n"), |acc, item| {
      let price = item.node.price;
      let amount = item.amount;

      // "{}\n{}: {} x {} pcs. = {} /del{}"
      let args = [&acc,
         &item.node.title,
         &price.to_string(),
         &amount.to_string(),
         &env::price_with_unit(price * amount),
         &item.node.id.to_string()
      ];
      loc(Key::BasketMakeOwnerText4, tag, &args)
   });

   node.title.clone() + descr.as_str() + time.as_str() + items.as_str()
}

fn markup(tag: LocaleTag) -> ReplyMarkup {
   let row1 = vec![
      loc::<char>(Key::BasketCommandEditName, tag, &[]),
      loc::<char>(Key::BasketCommandEditContact, tag, &[]),
      loc::<char>(Key::BasketCommandEditAddress, tag, &[]),
      loc::<char>(Key::BasketCommandEditDelivery, tag, &[]),
   ];
   let row2 = vec![
      loc::<char>(Key::BasketCommandReload, tag, &[]),
      loc::<char>(Key::BasketCommandClear, tag, &[]),
      loc::<char>(Key::BasketCommandExit, tag, &[]),
   ];

   let keyboard = vec![row1, row2];
   kb_markup(keyboard)
}

fn order_markup(node_id: i32, tag: LocaleTag) -> InlineKeyboardMarkup {
   let button = InlineKeyboardButton::callback(
      // "Checkout via bot"
      loc::<char>(Key::BasketOrderMarkup, tag, &[]), 
      format!("{}{}", cb::Command::TicketMake(0).as_ref(), node_id)
   );
   InlineKeyboardMarkup::default()
   .append_row(vec![button])
}
// ============================================================================
// [Fields editing mode]
// ============================================================================
#[derive(Clone)]
pub struct BasketStateEditing {
   prev_state: BasketState,
   cmd: EditCmd,
}

async fn enter_edit(bot: AutoSend<Bot>, msg: Message, state: BasketStateEditing) -> HandlerResult {

   let tag = state.prev_state.prev_state.locale;

   let (text, markup) = match state.cmd {
      EditCmd::Name => {
         // "Please {} indicate how the courier can contact you or press / to cancel"
         (loc(Key::BasketEnterEdit1, tag, &[&state.prev_state.customer.name]), cancel_markup())
      }
      EditCmd::Contact => 
      {
         // "If you want to allow staff to contact you directly, enter contacts (current value is '{}') or press / to cancel"
         (loc(Key::BasketEnterEdit2, tag, &[&state.prev_state.customer.contact]), cancel_markup())
      }
      EditCmd::Address => {
         let customer = &state.prev_state.customer;

         // Form a description of the address with a possible display of the geolocation
         let addr_desc = match customer.location_id() {
            Ok(message_id) => {
               // Try to forward geolocation message from history
               let from = msg.chat.id; // from bot
               let to = state.prev_state.prev_state.user_id; // to user
               let res = bot.forward_message(to, from, message_id).await;
               match res {
                  Ok(_) => loc::<char>(Key::BasketEnterEdit3, tag, &[]), // "previous location in the post above"
                  Err(_) => loc::<char>(Key::BasketEnterEdit4, tag, &[]), // "saved location is no longer available"
               }
            }
            Err(()) => {
               if customer.is_location() {
                  loc::<char>(Key::BasketEnterEdit4, tag, &[]) // "saved location is no longer available"
               } else {
                  loc(Key::BasketEnterEdit5, tag, &[&customer.address]) // "current address '{}'"
               }
            }
         };

         // "Enter the delivery address or point on the map (/ to cancel), {}. You can also send an arbitrary point or even broadcast its change, to do this, press the paperclip 📎 and select a geolocation."
         (loc(Key::BasketEnterEdit6, tag, &[&addr_desc]), address_markup(tag))
      }
      EditCmd::Delivery => {
         // "Current value is '{}', select delivery method"
         (loc(Key::BasketEnterEdit7, tag, &[&state.prev_state.customer.delivery_desc()]), delivery_markup())
      }
   };

   bot.send_message(msg.chat.id, text)
   .reply_markup(markup)
   .await?;

   Ok(())
}

pub async fn update_edit(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: BasketStateEditing) -> HandlerResult {
   async fn do_update(cmd: EditCmd, user_id: u64, ans: String, tag: LocaleTag) -> Result<String, String> {
      let cancel_command = loc::<char>(Key::BasketUpdateEdit1, tag, &[]); // "/"
      if ans == cancel_command {
         // "Cancel, value not changed",
         return Ok(loc::<char>(Key::BasketUpdateEdit2, tag, &[]));
      }

      // Store new value
      match cmd {
         EditCmd::Name => db::user_update_name(user_id, &ans).await?,
         EditCmd::Contact => db::user_update_contact(user_id, &ans).await?,
         EditCmd::Address => db::user_update_address(user_id, &ans).await?,
         EditCmd::Delivery => {
            // Parse answer
            let delivery = Delivery::from_str(ans.as_str());
            if delivery.is_err() {
               // "Error, delivery method not changed"
               return Ok(loc::<char>(Key::BasketUpdateEdit3, tag, &[]));
            }

            let delivery = delivery.unwrap();
            db::user_update_delivery(user_id, &delivery).await?;
         }
      }

      // "New value saved"
      Ok(loc::<char>(Key::BasketUpdateEdit4, tag, &[]))
   }

   let tag = state.prev_state.prev_state.locale;

   // Input may be text or geolocation
   let input = if let Some(input) = msg.text() {
      input.to_string()
   } else {
      if let Some(_) = msg.location() {
         Customer::make_location(msg.id)
      } else {
         String::default()
      }
   };

   // Report result
   let user_id = state.prev_state.prev_state.user_id.0 as u64;
   let text = do_update(state.cmd, user_id, input, tag).await?;

   bot.send_message(msg.chat.id, text).await?;

   // Reload node
   enter(bot, msg, dialogue, state.prev_state.prev_state).await
}

fn delivery_markup() -> ReplyMarkup {
   kb_markup(vec![vec![
      String::from(Delivery::Courier.as_ref()),
      String::from(Delivery::Pickup.as_ref())
   ]])
}

fn address_markup(tag: LocaleTag) -> ReplyMarkup {
   let kb = vec![
      KeyboardButton::new(loc::<char>(Key::BasketAddressMarkup, tag, &[])).request(ButtonRequest::Location), // "Geolocation"
      KeyboardButton::new(loc::<char>(Key::BasketUpdateEdit1, tag, &[])), // "/"
   ];

   let markup = KeyboardMarkup::new(vec![kb])
   .resize_keyboard(true);

   ReplyMarkup::Keyboard(markup)
}
