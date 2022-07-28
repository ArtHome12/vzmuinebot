/* ===============================================================================
Restaurant menu bot.
Basket menu. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, 
      ParseMode, ButtonRequest, InlineKeyboardButton, InlineKeyboardMarkup,
   }
};

use std::str::FromStr;
use strum::{AsRefStr, EnumString, EnumMessage, };
use enum_default::EnumDefault;

use crate::states::*;
use crate::database as db;
use crate::customer::*;
use crate::environment as env;
use crate::callback as cb;
use crate::node;
use crate::orders;
use crate::registration;
use crate::general;


// ============================================================================
// [Main entry]
// ============================================================================

// Main commands
#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "Очистить")]
   Clear, // add a new node
   #[strum(to_string = "Выход")]
   Exit, // return to start menu
   Edit(EditCmd),
   #[strum(to_string = "/del")]
   Delete(i32),
   #[strum(to_string = "⭮")]
   Reload,
   Unknown,
}

// Main commands
#[derive(Clone, AsRefStr, EnumString, EnumMessage, EnumDefault)]
enum EditCmd {
   #[strum(to_string = "Имя", message = "name")] // Button caption and db field name
   Name,
   #[strum(to_string = "Контакт", message = "contact")]
   Contact,
   #[strum(to_string = "Адрес", message = "address")]
   Address,
   #[strum(to_string = "Доставка", message = "delivery")]
   Delivery,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as edit subcommand
      if let Ok(edit) = EditCmd::from_str(s) {
         Self::Edit(edit)
      } else {
         // Try as main command
         Self::from_str(s)
         .unwrap_or_else(|_| {
            // Looking for the commands with arguments
            if s.get(..4).unwrap_or_default() == Self::Delete(0).as_ref() {
               let r_part = s.get(4..).unwrap_or_default();
               Command::Delete(r_part.parse().unwrap_or_default())
            } else {
               Command::Unknown
            }
         })
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
   // Start with info about user
   let info = format!("Ваши данные, {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      state.customer.name,
      state.customer.contact,
      state.customer.delivery_desc()
   );

   // Load info about orders
   let user_id = state.prev_state.user_id;
   let orders = db::orders(user_id.0 as i64).await?;

   // Announce
   let basket_info = orders.basket_info();
   let announce = if basket_info.orders_num == 0 {
      String::from("Корзина пуста")
   } else {
      format!("В корзине {} поз., {} шт. на общую сумму {}",
         basket_info.orders_num,
         basket_info.items_num,
         env::price_with_unit(basket_info.total_cost)
      )
   };
   bot.send_message(msg.chat.id, format!("{}\n\n{}", info, announce))
   .reply_markup(markup())
   .await?;

   // Messages by owners
   for owner in orders.data {
      let owner_id = owner.0.id;
      let text = make_owner_text(&owner.0, &owner.1);

      
      bot.send_message(msg.chat.id, text)
      .reply_markup(order_markup(owner_id))
      .parse_mode(ParseMode::Html)
      .await?;
   }

   // Show tickets (orders in process)
   registration::show_tickets(bot, state.prev_state.user_id).await?;

   Ok(())
}

pub async fn update(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: BasketState) -> HandlerResult {

   let user_id = state.prev_state.user_id.0 as u64;
   // Parse and handle commands
   let text = msg.text().unwrap_or_default();
   let cmd = Command::parse(text);
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
         bot.send_message(msg.chat.id, "Вы покидаете меню заказов").await?;

         // General commands handler - messaging, searching...
         general::update(bot, dialogue, text, state.prev_state, true).await
      }
   }
}

pub fn make_owner_text(node: &node::Node, order: &orders::Order) -> String {
   // Prepare info about owner
   let descr = if node.descr.len() <= 1 { String::default() } 
   else { format!("\n{}", node.descr) };

   let time = if node.time.0 == node.time.1 { String::from("\nВремя: круглосуточно") }
   else { format!("\nВремя: {}-{}", node.time.0.format("%H:%M"), node.time.1.format("%H:%M")) };

   // Info about items
   let items = order.iter()
   .fold(String::from("\n"), |acc, item| {
      let price = item.node.price;
      let amount = item.amount;

      format!("{}\n{}: {} x {} шт. = {} /del{}", acc,
         item.node.title,
         price,
         amount,
         env::price_with_unit(price * amount),
         item.node.id
      )
   });

   node.title.clone() + descr.as_str() + time.as_str() + items.as_str()
}

fn markup() -> ReplyMarkup {
   let row1 = vec![
      String::from(EditCmd::Name.as_ref()),
      String::from(EditCmd::Contact.as_ref()),
      String::from(EditCmd::Address.as_ref()),
      String::from(EditCmd::Delivery.as_ref()),
   ];
   let row2 = vec![
      String::from(Command::Reload.as_ref()),
      String::from(Command::Clear.as_ref()),
      String::from(Command::Exit.as_ref()),
   ];

   let keyboard = vec![row1, row2];
   kb_markup(keyboard)
}

fn order_markup(node_id: i32) -> InlineKeyboardMarkup {
   let button = InlineKeyboardButton::callback(
      String::from("Оформить через бота"), 
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
   let (text, markup) = match state.cmd {
      EditCmd::Name => (format!("Пожалуйста, {}, укажите как курьер может к Вам обращаться или нажмите / для отмены", state.prev_state.customer.name), cancel_markup()),
      EditCmd::Contact => (format!("Если хотите дать возможность ресторатору связаться с вами напрямую, укажите контакты (текущее значение '{}') или нажмите / для отмены", state.prev_state.customer.contact), cancel_markup()),
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
                  Ok(_) => String::from("прежняя геопозиция в сообщении выше"),
                  Err(_) => String::from("сохранённая геопозиция больше недоступна"),
               }
            }
            Err(()) => {
               if customer.is_location() {
                  String::from("сохранённая геопозиция больше недоступна")
               } else {
                  format!("текущий адрес '{}'", customer.address)
               }
            }
         };

         (format!("Введите адрес для доставки или укажите точку на карте (/ для отмены), {}. Также вы можете отправить произвольную точку или даже транслировать её изменение, для этого нажмите скрепку 📎 и выберите геопозицию.", 
         addr_desc),
         address_markup())
      }
      EditCmd::Delivery => (format!("Текущее значение '{}', выберите способ доставки", state.prev_state.customer.delivery_desc()), delivery_markup()),
   };

   bot.send_message(msg.chat.id, text)
   .reply_markup(markup)
   .await?;

   Ok(())
}

pub async fn update_edit(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: BasketStateEditing) -> HandlerResult {
   async fn do_update(cmd: EditCmd, user_id: u64, ans: String) -> Result<String, String> {
      if ans == String::from("/") {
         return Ok(String::from("Отмена, значение не изменено"));
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
               return Ok(String::from("Ошибка, способ доставки не изменён"));
            }

            let delivery = delivery.unwrap();
            db::user_update_delivery(user_id, &delivery).await?;
         }
      }

      Ok(String::from("Новое значение сохранено"))
   }

   // Report result
   let ans = msg.text().unwrap_or_default().to_string();
   let user_id = state.prev_state.prev_state.user_id.0 as u64;
   let text = do_update(state.cmd, user_id, ans).await?;

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

fn address_markup() -> ReplyMarkup {
   let kb = vec![
      KeyboardButton::new("Геопозиция").request(ButtonRequest::Location),
      KeyboardButton::new("/"),
   ];

   let markup = KeyboardMarkup::new(vec![kb])
   .resize_keyboard(true);

   ReplyMarkup::Keyboard(markup)
}
