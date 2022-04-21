/* ===============================================================================
Restaurant menu bot.
Basket menu. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*, dispatching::dialogue::InMemStorage,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, 
      ParseMode, ButtonRequest, InlineKeyboardButton, InlineKeyboardMarkup,
   }
};
use teloxide_macros::DialogueState;
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

type MyDialogue = Dialogue<Command, InMemStorage<Command>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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
   pub state: CommandState,
   pub customer: Customer,
}

/* async fn update(state: BasketState, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {

   // Parse and handle commands
   let cmd = Command::parse(ans.as_str());
   match cmd {
      Command::Clear => {
         db::orders_delete(state.state.user_id)
         .await
         .map_err(|s| map_req_err(s))?;

         // Reload basket
         enter(state.state, cx).await
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, String::default()).await,

      Command::Edit(cmd) => enter_edit(BasketStateEditing { state, cmd }, cx).await,

      Command::Delete(node_id) => {
         db::order_delete_node(state.state.user_id, node_id)
         .await
         .map_err(|s| map_req_err(s))?;

         // Reload basket
         enter(state.state, cx).await
      }

      Command::Reload => enter(state.state, cx).await,

      Command::Unknown => {
         cx.answer("Вы покидаете меню заказов")
         .await?;

         // General commands handler - messaging, searching...
         general::update(state.state, cx, ans, true).await
      }
   }
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Load user info
   let customer = db::user(state.user_id).await
   .map_err(|s| map_req_err(s))?;

   // Display
   let state = BasketState { state, customer };
   view(state, cx).await
}

pub async fn view(state: BasketState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {
   // Start with info about user
   let info = format!("Ваши данные, {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      state.customer.name,
      state.customer.contact,
      state.customer.delivery_desc()
   );

   // Load info about orders
   let user_id = state.state.user_id;
   let orders = db::orders(user_id)
   .await
   .map_err(|s| map_req_err(s))?;

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
   cx.answer(format!("{}\n\n{}", info, announce))
   .reply_markup(markup())
   .await?;

   // Messages by owners
   for owner in orders.data {
      let owner_id = owner.0.id;
      let text = make_owner_text(&owner.0, &owner.1);

      cx.answer(text)
      .reply_markup(order_markup(owner_id))
      .parse_mode(ParseMode::Html)
      .await?;
   }

   // Show tickets (orders in process)
   registration::show_tickets(state.state.user_id, cx).await?;

   next(state)
}*/

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
   state: BasketState,
   cmd: EditCmd,
}

/*async fn update_edit(mut state: BasketStateEditing, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {
   async fn do_update(state: &mut BasketStateEditing, ans: String) -> Result<String, String> {
      if ans == String::from("/") {
         return Ok(String::from("Отмена, значение не изменено"));
      }

      // Store new value
      let user_id = state.state.state.user_id;
      match state.cmd {
         EditCmd::Name => {
            db::user_update_name(user_id, &ans).await?;
            state.state.customer.name = ans;
         }
         EditCmd::Contact => {
            db::user_update_contact(user_id, &ans).await?;
            state.state.customer.contact = ans;
         }
         EditCmd::Address => {
            db::user_update_address(user_id, &ans).await?;
            state.state.customer.address = ans;
         }
         EditCmd::Delivery => {
            // Parse answer
            let delivery = Delivery::from_str(ans.as_str());
            if delivery.is_err() {
               return Ok(String::from("Ошибка, способ доставки не изменён"));
            }

            let delivery = delivery.unwrap();
            db::user_update_delivery(user_id, &delivery).await?;
            state.state.customer.delivery = delivery;
         }
      }

      Ok(String::from("Новое значение сохранено"))
   }

   // Report result
   let info = do_update(&mut state, ans)
   .await
   .map_err(|s| map_req_err(s))?;

   cx.answer(info)
   .await?;

   // Reload node
   view(state.state, cx).await
}

async fn enter_edit(state: BasketStateEditing, cx: TransitionIn<AutoSend<Bot>>) -> TransitionOut<Dialogue> {
   let (info, markup) = match state.cmd {
      EditCmd::Name => (format!("Пожалуйста, {}, укажите как курьер может к Вам обращаться или нажмите / для отмены", state.state.customer.name), cancel_markup()),
      EditCmd::Contact => (format!("Если хотите дать возможность ресторатору связаться с вами напрямую, укажите контакты (текущее значение '{}') или нажмите / для отмены", state.state.customer.contact), cancel_markup()),
      EditCmd::Address => {
         let customer = &state.state.customer;

         // Form a description of the address with a possible display of the geolocation
         let addr_desc = match customer.location_id() {
            Ok(message_id) => {
               // Try to forward geolocation message from history
               let from = cx.update.chat_id(); // from bot
               let to = state.state.state.user_id; // to user
               let res = cx.requester.forward_message(to, from, message_id).await;
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
      EditCmd::Delivery => (format!("Текущее значение '{}', выберите способ доставки", state.state.customer.delivery_desc()), delivery_markup()),
   };

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}*/

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
