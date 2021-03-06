/* ===============================================================================
Restaurant menu bot.
Registration orders. 07 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{RequestError, payloads::SendMessageSetters, prelude::*, types::{CallbackQuery, ParseMode, }};
use regex::Regex;
use lazy_static::lazy_static;
use strum::EnumMessage;


use crate::database as db;
use crate::customer::*;
use crate::node;
use crate::ticket::*;
use crate::environment as env;
use crate::states;


type Update = UpdateWithCx<AutoSend<Bot>, CallbackQuery>;
type ResultMessage = Result<Message, String>;
type Result3Id = Result<ThreeMsgId, String>;

const VALID_USER_ID: i64 = 10_000;

pub async fn make_ticket(cx: &Update, node_id: i32) -> Result<&'static str, String> {

   // Load customer info
   let user_id = cx.update.from.id;
   let customer = db::user(user_id).await?;

   // Load owners node
   let node = db::node(db::LoadNode::EnabledIdNoChildren(node_id)).await?;
   let owners = if let Some(node) = node { node.owners } else { node::Owners::default() };

   // Check valid owner
   if owners.0 < VALID_USER_ID && owners.1 < VALID_USER_ID && owners.2 < VALID_USER_ID {
      let text = "Заведение пока не подключено к боту, пожалуйста скопируйте ваш заказ отправьте по указанным контактным данным напрямую, после чего можно очистить корзину";
      reply_msg(user_id, cx, text).await?;
      return Ok("Неудачно");
   }

   // Get source message text and id
   let old_text = cx.update.message.clone()
   .and_then(|f| f.text()
      .and_then(|f| Some(f.to_string()))
   );
   if old_text.is_none() {
      let text = "Не удаётся получить текст заказа, возможно слишком старое сообщение";
      reply_msg(user_id, cx, text).await?;
      return Ok("Неудачно");
   }
   let old_text = old_text.unwrap();
   let orig_msg_id = cx.update.message.clone().unwrap().id; // unwrap checked above

   // Check delivery address if not pickup
   if matches!(customer.delivery, Delivery::Courier) {

      match customer.is_location() {
         true => {
            // Send to owner a message with the geographic location to make sure it's still available
            let message_id = customer.location_id().unwrap_or_default();
            let res = forward_msg_to_owners(&owners, cx, message_id).await;
            if let Err(err) = res {
               let text = format!("Недоступно сообщение с геопозицией, пожалуйста обновите адрес\n<i>{}</i>", err);
               reply_msg(user_id, cx, &text).await?;
               return Ok("Неудачно");
            }
         }

         false => {
            if customer.address.len() < 1 {
               let text = "Пожалуйста, введите адрес или переключитесь на самовывоз при помощи кнопок внизу.\nЭта информация будет сохранена для последующих заказов, при необходимости вы всегда сможете её изменить";
               reply_msg(user_id, cx, text).await?;
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

   let cust_msg_id = cx.requester.edit_message_text(user_id, orig_msg_id, &order_info)
   .await
   .map_err(|err| format!("make_ticket edit_message user_id={} {}", user_id, err))?.id;

   // Send to owner info about customer
   let customer_info = format!("Заказ от {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      customer.name,
      customer.contact,
      customer.delivery_desc()
   );
   send_msg_to_owners(&owners, cx, &customer_info).await?;

   // Forward edited message with order and save msg id
   let owners_msg_id = forward_msg_to_owners(&owners, cx, orig_msg_id).await?;

   // Send the order also to the service chat
   let service_msg_id = env::log(&format!("{}\n---\n{}", customer_info, order_info)).await;

   // Delete data from orders and create ticket with owners
   let ticket = db::ticket_form_orders(node_id, user_id, owners_msg_id, cust_msg_id, service_msg_id).await?;

   let t = TicketWithOwners {
      ticket,
      owners,
   };

   // Send messages with status to customer and owners
   update_statuses(&cx, t).await?;

   Ok("Успешно")
}

async fn update_statuses(cx: &Update, mut t: TicketWithOwners) -> Result<(), String> {

   // The status change for customer is mandatory
   t.ticket.cust_status_msg_id = update_status(&cx.requester, &mut t, Role::Customer).await?;

   // Update status for owners, ignore fail
   t.ticket.owners_status_msg_id.0 = update_status(&cx.requester, &mut t, Role::Owner1)
   .await
   .ok()
   .flatten();

   // The same for owner 2
   t.ticket.owners_status_msg_id.1 = update_status(&cx.requester, &mut t, Role::Owner2)
   .await
   .ok()
   .flatten();

   // The same for owner 3
   t.ticket.owners_status_msg_id.2 = update_status(&cx.requester, &mut t, Role::Owner3)
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

enum Role {
   Customer,
   Owner1,
   Owner2,
   Owner3,
}

async fn update_status(bot: &AutoSend<Bot>, t: &mut TicketWithOwners, role: Role) -> Result<Option<i32>, String> {
   // Prepare data
   let (receiver, order_msg_id, status_msg_id) = match role {
      Role::Customer => (t.ticket.customer_id, Some(t.ticket.cust_msg_id), t.ticket.cust_status_msg_id),
      Role::Owner1 => (t.owners.0, t.ticket.owners_msg_id.0, t.ticket.owners_status_msg_id.0),
      Role::Owner2 => (t.owners.1, t.ticket.owners_msg_id.1, t.ticket.owners_status_msg_id.1),
      Role::Owner3 => (t.owners.2, t.ticket.owners_msg_id.2, t.ticket.owners_status_msg_id.2),
   };

   if receiver < VALID_USER_ID {
      return Ok(None);
   }

   // Text and markup for status message
   let info_for = match role {
      Role::Customer => InfoFor::Customer,
      _ => InfoFor::Owner,
   };
   let text = t.stage_message(info_for);
   let markup = t.ticket.make_markup(info_for);

   // Not all owners can exist and, accordingly, there are no message codes
   if order_msg_id.is_none() {
      let err = format!("registration::update_status order_msg_id is none for owner_id={}", receiver);
      return Err(err);
   }

   // Delete previous message with status
   if let Some(msg_id) = status_msg_id {
      let res = bot.delete_message(receiver, msg_id)
      .await;

      // Failure to delete a message is a normal situation, but fail notification is not
      if res.is_err() {
         let text = "Невозможно удалить предыдущее сообщение со статусом заказа, возможно оно уже было удалено";
         bot.send_message(receiver, text)
         .await
         .map_err(|err| format!("registration::update_status::delete old status user_id={}: {}", receiver, err))?;
      }
   }

   // Quote order message with current stage and commands. The receiver's validity is guaranteed in the previous step
   let mut res = bot.send_message(receiver, text)
   .reply_to_message_id(order_msg_id.unwrap());

   if let Some(markup) = markup { res = res.reply_markup(markup) }

   let res = res.await
   .map_err(|err| format!("registration::update_status user_id={}: {}", receiver, err))?;

   Ok(Some(res.id))
}

pub async fn cancel_ticket(cx: &Update, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = if cx.update.from.id == t.ticket.customer_id {
      Stage::CanceledByCustomer
   } else {
      Stage::CanceledByOwner
   };
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   let stage = t.ticket.stage;
   update_statuses(cx, t).await?;

   // Send the status also to the service chat
   let status = stage.get_message().unwrap();
   env::log_reply(status, service_msg_id).await;

   Ok("Успешно")
}

pub async fn next_ticket(cx: &Update, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   if t.ticket.next_stage() {
      // Update status in database if it was really changed
      db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;
   }

   update_statuses(cx, t).await?;
   Ok("Успешно")
}

pub async fn confirm_ticket(cx: &Update, ticket_id: i32) -> Result<&'static str, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = Stage::Finished;
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   update_statuses(cx, t).await?;

   // Send the order also to the service chat
   let status = "Заказ успешно завершён";
   env::log_reply(status, service_msg_id).await;

   Ok("Успешно")
}

async fn reply_msg(receiver: i64, cx: &Update, text: &str) -> Result<(), String> {
   let mut ans = cx.requester
   .send_message(receiver, text)
   .parse_mode(ParseMode::Html);

   if let Some(reply_to_id) = &cx.update.message {
      ans = ans.reply_to_message_id(reply_to_id.id);
   }

   ans.await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(())
}

async fn send_msg(receiver: i64, cx: &Update, text: &str) -> ResultMessage {
   let res = cx.requester
   .send_message(receiver, text)
   .parse_mode(ParseMode::Html)
   .await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(res)
}

async fn forward_msg(receiver: i64, cx: &Update, message_id: i32) -> ResultMessage {
   let from = &cx.update.message;
   if from.is_none() {
      Err(format!("registration::forward_msg none cx.update.message for receiver={}", receiver))
   } else {
      let from = from.clone().unwrap().chat_id();
      let res = cx.requester.forward_message(receiver, from, message_id).await
      .map_err(|err| format!("forward_msg {}", err))?;
      Ok(res)
   }
}

async fn send_msg_to_owners(owners: &node::Owners, cx: &Update, text: &str) -> Result3Id {
   // Try to send to all owners
   let owner1 = send_msg(owners.0, cx, text).await;
   let owner2 = send_msg(owners.1, cx, text).await;
   let owner3 = send_msg(owners.2, cx, text).await;

   // Report an error from owner1 if there are no successful attempts
   unwrap_msg_id(owner1, owner2, owner3)
}

async fn forward_msg_to_owners(owners: &node::Owners, cx: &Update, message_id: i32) -> Result3Id {
   // Try to send to all owners
   let owner1 = forward_msg(owners.0, cx, message_id).await;
   let owner2 = forward_msg(owners.1, cx, message_id).await;
   let owner3 = forward_msg(owners.2, cx, message_id).await;
   
   unwrap_msg_id(owner1, owner2, owner3)
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

pub async fn show_tickets(user_id: i64, cx: TransitionIn<AutoSend<Bot>>,) -> Result<(), RequestError> {
   let tickets = db::tickets(user_id)
   .await
   .map_err(|s| states::map_req_err(s))?;

   for mut t in tickets {
      
      // Detect own role - the owner or client
      let role = if user_id == t.ticket.customer_id { Role::Customer }
      else if user_id == t.owners.0 { Role::Owner1 }
      else if user_id == t.owners.1 { Role::Owner2 }
      else if user_id == t.owners.2 { Role::Owner3 }
      else {
         let e = format!("registration::show_tickets user_id={}: unknown role", user_id);
         return Err(states::map_req_err(e));
      };

      // Update status - code like update_statuses()
      match role {
         Role::Customer => {
            t.ticket.cust_status_msg_id =
            update_status(&cx.requester, &mut t, Role::Customer)
            .await
            // The status change for customer is mandatory
            .map_err(|err| states::map_req_err(format!("registration::show_tickets {}", err)))?
         }
         Role::Owner1 => {
            t.ticket.owners_status_msg_id.0 =
            update_status(&cx.requester, &mut t, Role::Owner1)
            .await
            .ok()
            .flatten()
         }
         Role::Owner2 => {
            t.ticket.owners_status_msg_id.1 =
            update_status(&cx.requester, &mut t, Role::Owner2)
            .await
            .ok()
            .flatten()
         }
         Role::Owner3 => {
            t.ticket.owners_status_msg_id.2 =
            update_status(&cx.requester, &mut t, Role::Owner3)
            .await
            .ok()
            .flatten()
         }
      }

      // Update status id on database
      db::ticket_update_status_messages(&t.ticket)
      .await
      .map_err(|err| states::map_req_err(format!("registration::show_tickets {}", err)))?
   }

   Ok(())
}