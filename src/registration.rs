/* ===============================================================================
Restaurant menu bot.
Registration orders. 07 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*, payloads::SendMessageSetters,
   types::{CallbackQuery, ParseMode, Recipient, ChatId, UserId,}
};
use regex::Regex;
use lazy_static::lazy_static;
use strum::EnumMessage;


use crate::database as db;
use crate::customer::*;
use crate::node;
use crate::ticket::*;
use crate::environment as env;


/* type Update = UpdateWithCx<AutoSend<Bot>, CallbackQuery>; */
type ResultMessage = Result<Message, String>;
type Result3Id = Result<ThreeMsgId, String>;

const VALID_USER_ID: u64 = 10_000;

enum Role {
   Customer,
   Owner1,
   Owner2,
   Owner3,
}

pub async fn show_tickets(bot: AutoSend<Bot>, user_id: UserId) -> Result<(), String> {
   let tickets = db::tickets(user_id.0 as i64).await?;

   let user_id = user_id.0 as i64;

   for mut t in tickets {
      
      // Detect own role - the owner or client
      let role = if user_id == t.ticket.customer_id { Role::Customer }
      else if user_id == t.owners.0 { Role::Owner1 }
      else if user_id == t.owners.1 { Role::Owner2 }
      else if user_id == t.owners.2 { Role::Owner3 }
      else {
         let e = format!("registration::show_tickets user_id={}: unknown role", user_id);
         return Err(e);
      };

      // Update status - code like update_statuses()
      match role {
         Role::Customer => {
            t.ticket.cust_status_msg_id =
               update_status(&bot, &mut t, Role::Customer).await?
         }
         Role::Owner1 => {
            t.ticket.owners_status_msg_id.0 =
               update_status(&bot, &mut t, Role::Owner1)
               .await
               .ok()
               .flatten()
         }
         Role::Owner2 => {
            t.ticket.owners_status_msg_id.1 =
               update_status(&bot, &mut t, Role::Owner2)
               .await
               .ok()
               .flatten()
         }
         Role::Owner3 => {
            t.ticket.owners_status_msg_id.2 =
               update_status(&bot, &mut t, Role::Owner3)
               .await
               .ok()
               .flatten()
         }
      }

      // Update status id on database
      db::ticket_update_status_messages(&t.ticket).await?
   }

   Ok(())
}

async fn update_status(bot: &AutoSend<Bot>, t: &mut TicketWithOwners, role: Role) -> Result<Option<i32>, String> {
   // Prepare data
   let (recipient_id, order_msg_id, status_msg_id) = match role {
      Role::Customer => (t.ticket.customer_id, Some(t.ticket.cust_msg_id), t.ticket.cust_status_msg_id),
      Role::Owner1 => (t.owners.0, t.ticket.owners_msg_id.0, t.ticket.owners_status_msg_id.0),
      Role::Owner2 => (t.owners.1, t.ticket.owners_msg_id.1, t.ticket.owners_status_msg_id.1),
      Role::Owner3 => (t.owners.2, t.ticket.owners_msg_id.2, t.ticket.owners_status_msg_id.2),
   };

   if recipient_id < VALID_USER_ID as i64 {
      return Ok(None);
   }
   let recipient = Recipient::Id(ChatId(recipient_id));

   // Text and markup for status message
   let info_for = match role {
      Role::Customer => InfoFor::Customer,
      _ => InfoFor::Owner,
   };
   let text = t.stage_message(info_for);
   let markup = t.ticket.make_markup(info_for);

   // Not all owners can exist and, accordingly, there are no message codes
   if order_msg_id.is_none() {
      let err = format!("registration::update_status order_msg_id is none for owner_id={}", recipient);
      return Err(err);
   }

   // Delete previous message with status
   if let Some(msg_id) = status_msg_id {
      let res = bot.delete_message(recipient.to_owned(), msg_id)
      .await;

      // Failure to delete a message is a normal situation, but fail notification is not
      if res.is_err() {
         let text = "Невозможно удалить предыдущее сообщение со статусом заказа, возможно оно уже было удалено";
         bot.send_message(recipient.to_owned(), text)
         .await
         .map_err(|err| format!("registration::update_status::delete old status user_id={}: {}", recipient_id, err))?;
      }
   }

   // Quote order message with current stage and commands. The receiver's validity is guaranteed in the previous step
   let mut res = bot.send_message(recipient, text)
   .reply_to_message_id(order_msg_id.unwrap());

   if let Some(markup) = markup { res = res.reply_markup(markup) }

   let res = res.await
   .map_err(|err| format!("registration::update_status user_id={}: {}", recipient_id, err))?;

   Ok(Some(res.id))
}


pub async fn make_ticket(bot: &AutoSend<Bot>, q: CallbackQuery, node_id: i32) -> Result<&'static str, String> {

   // Load customer info
   let user_id = q.from.id;
   let customer = db::user(user_id.0).await?;

   // Load owners node
   let node = db::node(db::LoadNode::EnabledIdNoChildren(node_id)).await?;
   let owners = if let Some(node) = node { node.owners } else { node::Owners::default() };

   let reply_to_id = if let Some(msg) = &q.message { msg.id } else { 0 };

   // Check valid owner
   if owners.0 < VALID_USER_ID as i64 && owners.1 < VALID_USER_ID as i64 && owners.2 < VALID_USER_ID as i64 {
      let text = "Заведение пока не подключено к боту, пожалуйста скопируйте ваш заказ отправьте по указанным контактным данным напрямую, после чего можно очистить корзину";
      reply_msg(bot, user_id, reply_to_id, text).await?;
      return Ok("Неудачно");
   }

   // Get source message text and id
   let ref_m = q.message.as_ref();
   let old_text = ref_m.and_then(|f| f.text()
      .and_then(|f| Some(f.to_string()))
   );
   if old_text.is_none() {
      let text = "Не удаётся получить текст заказа, возможно слишком старое сообщение";
      reply_msg(bot, user_id, reply_to_id, text).await?;
      return Ok("Неудачно");
   }
   let old_text = old_text.unwrap();
   let orig_msg_id = ref_m.unwrap().id; // unwrap checked above

   // Check delivery address if not pickup
   if matches!(customer.delivery, Delivery::Courier) {

      match customer.is_location() {
         true => {
            // Send to owner a message with the geographic location to make sure it's still available
            let message_id = customer.location_id().unwrap_or_default();
            let res = forward_msg_to_owners(&bot, user_id, &owners, message_id).await;
            if let Err(err) = res {
               let text = format!("Недоступно сообщение с геопозицией, пожалуйста обновите адрес\n<i>{}</i>", err);
               reply_msg(bot, user_id, reply_to_id, &text).await?;
               return Ok("Неудачно");
            }
         }

         false => {
            if customer.address.len() < 1 {
               let text = "Пожалуйста, введите адрес или переключитесь на самовывоз при помощи кнопок внизу.\nЭта информация будет сохранена для последующих заказов, при необходимости вы всегда сможете её изменить";
               reply_msg(bot, user_id, reply_to_id, &text).await?;
               return Ok("Неудачно");
            }
         }
      }
   }

   // Edit the original message - remove commands from text
   lazy_static! {
      static ref HASHTAG_REGEX : Regex = Regex::new(r" /del\d+").unwrap();
   }
   let order_info = HASHTAG_REGEX.replace_all(&old_text, "").to_string();

   let cust_msg_id = bot.edit_message_text(user_id, orig_msg_id, &order_info)
   .await
   .map_err(|err| format!("make_ticket edit_message user_id={} {}", user_id, err))?.id;

   // Send to owner info about customer
   let customer_info = format!("Заказ от {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      customer.name,
      customer.contact,
      customer.delivery_desc()
   );
   send_msg_to_owners(&bot, &owners, &customer_info).await?;

   // Forward edited message with order and save msg id
   let owners_msg_id = forward_msg_to_owners(&bot, user_id, &owners, orig_msg_id).await?;

   // Send the order also to the service chat
   let service_msg_id = env::log(&format!("{}\n---\n{}", customer_info, order_info)).await;

   // Delete data from orders and create ticket with owners
   let ticket = db::ticket_form_orders(node_id, user_id.0 as i64, owners_msg_id, cust_msg_id, service_msg_id).await?;

   let t = TicketWithOwners {
      ticket,
      owners,
   };

   // Send messages with status to customer and owners
   update_statuses(bot, t).await?;

   Ok("Успешно")
}

async fn update_statuses(bot: &AutoSend<Bot>, mut t: TicketWithOwners) -> Result<(), String> {

   // The status change for customer is mandatory
   t.ticket.cust_status_msg_id = update_status(bot, &mut t, Role::Customer).await?;

   // Update status for owners, ignore fail
   t.ticket.owners_status_msg_id.0 = update_status(bot, &mut t, Role::Owner1)
   .await
   .ok()
   .flatten();

   // The same for owner 2
   t.ticket.owners_status_msg_id.1 = update_status(bot, &mut t, Role::Owner2)
   .await
   .ok()
   .flatten();

   // The same for owner 3
   t.ticket.owners_status_msg_id.2 = update_status(bot, &mut t, Role::Owner3)
   .await
   .ok()
   .flatten();

   // The status change for the owner must be for at least one
   let res = t.ticket.owners_status_msg_id.0.or(t.ticket.owners_status_msg_id.1).or(t.ticket.owners_status_msg_id.2);
   if res.is_none() {
      let err = format!("registration::next_ticket user_id={}: all owners notification fail", t.ticket.customer_id);
      return Err(err);
   }

   // Update status id on database
   db::ticket_update_status_messages(&t.ticket).await?;
   Ok(())
}

pub async fn cancel_ticket(bot: &AutoSend<Bot>, q: CallbackQuery, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = if q.from.id.0 as i64 == t.ticket.customer_id {
      Stage::CanceledByCustomer
   } else {
      Stage::CanceledByOwner
   };
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   let stage = t.ticket.stage;
   update_statuses(bot, t).await?;

   // Send the status also to the service chat
   let status = stage.get_message().unwrap();
   env::log_reply(status, service_msg_id).await;

   Ok("Успешно")
}

pub async fn next_ticket(bot: &AutoSend<Bot>, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   if t.ticket.next_stage() {
      // Update status in database if it was really changed
      db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;
   }

   update_statuses(bot, t).await?;
   Ok("Успешно")
}

pub async fn confirm_ticket(bot: &AutoSend<Bot>, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = Stage::Finished;
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   update_statuses(bot, t).await?;

   // Send the order also to the service chat
   let status = "Заказ успешно завершён";
   env::log_reply(status, service_msg_id).await;

   Ok("Успешно")
}

async fn reply_msg(bot: &AutoSend<Bot>, receiver: UserId, reply_to_id: i32, text: &str) -> Result<(), String> {
   let mut fut = bot.send_message(receiver, text)
   .parse_mode(ParseMode::Html);

   if reply_to_id != 0 {
      fut = fut.reply_to_message_id(reply_to_id);
   }

   fut.await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(())
}


async fn send_msg(bot: &AutoSend<Bot>, receiver: UserId, text: &str) -> ResultMessage {
   let res = bot.send_message(receiver, text)
   .parse_mode(ParseMode::Html)
   .await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(res)
}

async fn forward_msg(bot: &AutoSend<Bot>, from: UserId, receiver: UserId, message_id: i32) -> ResultMessage {
   let res = bot.forward_message(receiver, from, message_id).await
   .map_err(|err| format!("forward_msg {}", err))?;
   Ok(res)
}

fn owners_to_users(owners: &node::Owners) -> (UserId, UserId, UserId) {
   let user1 = UserId(owners.0 as u64);
   let user2 = UserId(owners.1 as u64);
   let user3 = UserId(owners.2 as u64);
   (user1, user2, user3)
}

async fn send_msg_to_owners(bot: &AutoSend<Bot>, owners: &node::Owners, text: &str) -> Result3Id {
   // Try to send to all owners
   let users = owners_to_users(owners);
   let msg1 = send_msg(bot, users.0, text).await;
   let msg2 = send_msg(bot, users.1, text).await;
   let msg3 = send_msg(bot, users.2, text).await;

   // Report an error from owner1 if there are no successful attempts
   unwrap_msg_id(msg1, msg2, msg3)
}

async fn forward_msg_to_owners(bot: &AutoSend<Bot>, from: UserId, owners: &node::Owners, message_id: i32) -> Result3Id {
   // Try to send to all owners
   let users = owners_to_users(owners);
   let msg1 = forward_msg(bot, from, users.0, message_id).await;
   let msg2 = forward_msg(bot, from, users.1, message_id).await;
   let msg3 = forward_msg(bot, from, users.2, message_id).await;
   
   unwrap_msg_id(msg1, msg2, msg3)
}

fn unwrap_msg_id(msg1: ResultMessage, msg2: ResultMessage, msg3: ResultMessage) -> Result3Id {
   // Report an error from msg1 if there are no successful attempts
   if msg1.is_err() && msg2.is_err() && msg3.is_err() {
      return Err(msg1.unwrap_err());
   }

   let res: ThreeMsgId = (
      msg1.and_then(|op| Ok(op.id)).ok(),
      msg2.and_then(|op| Ok(op.id)).ok(),
      msg3.and_then(|op| Ok(op.id)).ok()
   );
   Ok(res)
}
