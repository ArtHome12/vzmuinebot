/* ===============================================================================
Restaurant menu bot.
Registration orders. 07 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*, payloads::SendMessageSetters,
   types::{CallbackQuery, ParseMode, Recipient, ChatId, UserId, MessageId}
};
use regex::Regex;
use lazy_static::lazy_static;

use crate::database as db;
use crate::customer::*;
use crate::node;
use crate::ticket::*;
use crate::environment as env;
use crate::loc::*;


/* type Update = UpdateWithCx<Bot, CallbackQuery>; */
type ResultMessage = Result<Message, String>;
type Result3Id = Result<ThreeMsgId, String>;


enum Role {
   Customer,
   Owner1,
   Owner2,
   Owner3,
}

pub async fn show_tickets(bot: Bot, user_id: UserId, tag: LocaleTag) -> Result<(), String> {
   let tickets = db::tickets(user_id.0 as i64).await?;

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
               update_status(&bot, &mut t, Role::Customer, tag).await?
         }
         Role::Owner1 => {
            t.ticket.owners_status_msg_id.0 =
               update_status(&bot, &mut t, Role::Owner1, tag)
               .await
               .ok()
               .flatten()
         }
         Role::Owner2 => {
            t.ticket.owners_status_msg_id.1 =
               update_status(&bot, &mut t, Role::Owner2, tag)
               .await
               .ok()
               .flatten()
         }
         Role::Owner3 => {
            t.ticket.owners_status_msg_id.2 =
               update_status(&bot, &mut t, Role::Owner3, tag)
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

async fn update_status(bot: &Bot, t: &mut TicketWithOwners, role: Role, tag: LocaleTag) -> Result<Option<MessageId>, String> {
   // Prepare data
   let (recipient_id, order_msg_id, status_msg_id) = match role {
      Role::Customer => (t.ticket.customer_id, Some(t.ticket.cust_msg_id), t.ticket.cust_status_msg_id),
      Role::Owner1 => (t.owners.0, t.ticket.owners_msg_id.0, t.ticket.owners_status_msg_id.0),
      Role::Owner2 => (t.owners.1, t.ticket.owners_msg_id.1, t.ticket.owners_status_msg_id.1),
      Role::Owner3 => (t.owners.2, t.ticket.owners_msg_id.2, t.ticket.owners_status_msg_id.2),
   };

   if recipient_id.0 < node::Owners::VALID_USER_ID {
      return Ok(None);
   }
   let recipient = Recipient::Id(ChatId(recipient_id.0 as i64));

   // Text and markup for status message
   let info_for = match role {
      Role::Customer => InfoFor::Customer,
      _ => InfoFor::Owner,
   };
   let text = t.stage_message(info_for, tag);
   let markup = t.ticket.make_markup(info_for, tag);

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
         // "Unable to delete previous order status message, it may have already been deleted"
         let text = loc(Key::RegUpdateStatus, tag, &[]);
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


pub async fn make_ticket(bot: &Bot, q: CallbackQuery, node_id: i32, tag: LocaleTag) -> Result<String, String> {

   // Load customer info
   let user_id = q.from.id;
   let customer = db::user(user_id.0).await?;

   // Load owners node
   let node = db::node(db::LoadNode::EnabledIdNoChildren(node_id)).await?;
   let owners = if let Some(node) = node { node.owners } else { node::Owners::default() };

   let reply_to_id = if let Some(msg) = &q.message { msg.id } else { MessageId(0) };

   // Check valid owner
   if !owners.has_valid_owner() {
      // "The place is not yet connected to the bot, please copy your order and send it directly to the specified contact details, after which you can empty the cart"
      let text = loc(Key::RegMakeTicket1, tag, &[]);
      reply_msg(bot, user_id, reply_to_id, &text).await?;
      // "Unsuccessfully"
      return Ok(loc(Key::RegMakeTicket2, tag, &[]));
   }

   // Get source message text and id
   let ref_m = q.message.as_ref();
   let old_text = ref_m.and_then(|f| f.text()
      .and_then(|f| Some(f.to_string()))
   );
   if old_text.is_none() {
      // "Unable to get order text, message may be too old"
      let text = loc(Key::RegMakeTicket3, tag, &[]);
      reply_msg(bot, user_id, reply_to_id, &text).await?;
      // "Unsuccessfully"
      return Ok(loc(Key::RegMakeTicket2, tag, &[]));
   }
   let old_text = old_text.unwrap();
   let orig_msg_id = ref_m.unwrap().id; // unwrap checked above

   // Check delivery address if not pickup
   if matches!(customer.delivery, Delivery::Courier) {

      match customer.is_location() {
         true => {
            // Send to owner a message with the geographic location to make sure it's still available
            let message_id = customer.location_id().unwrap_or(MessageId(0));
            let res = forward_msg_to_owners(&bot, user_id, &owners, message_id).await;
            if let Err(err) = res {
               // "Location message unavailable, please update address\n<i>{}</i>"
               let text = loc(Key::RegMakeTicket4, tag, &[&err]);
               reply_msg(bot, user_id, reply_to_id, &text).await?;
               // "Unsuccessfully"
               return Ok(loc(Key::RegMakeTicket2, tag, &[]));
            }
         }

         false => {
            if customer.address.len() < 1 {
               // "Please enter an address or switch to pickup using the buttons below.\nThis information will be saved for future orders, you can always change it if necessary"
               let text = loc(Key::RegMakeTicket5, tag, &[]);
               reply_msg(bot, user_id, reply_to_id, &text).await?;
               // "Unsuccessfully"
               return Ok(loc(Key::RegMakeTicket2, tag, &[]));
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

   // Send to owner info about customer "Order from {}:\nContact for communication: {}\nDelivery method: {}"
   let customer_info = loc(Key::RegMakeTicket6, tag, &[
      &customer.name,
      &customer.contact,
      &customer.delivery_desc(tag)
   ]);
   send_msg_to_owners(&bot, &owners, &customer_info).await?;

   // Forward edited message with order and save msg id
   let owners_msg_id = forward_msg_to_owners(&bot, user_id, &owners, orig_msg_id).await?;

   // Send the order also to the service chat
   let service_msg_id = env::log(&format!("{}\n---\n{}", customer_info, order_info)).await;

   // Delete data from orders and create ticket with owners
   let ticket = db::ticket_form_orders(node_id, user_id, owners_msg_id, cust_msg_id, service_msg_id).await?;

   let t = TicketWithOwners {
      ticket,
      owners,
   };

   // Send messages with status to customer and owners
   update_statuses(bot, t, tag).await?;

   // "Successfully"
   Ok(loc(Key::RegMakeTicket7, tag, &[]))
}

async fn update_statuses(bot: &Bot, mut t: TicketWithOwners, tag: LocaleTag) -> Result<(), String> {

   // The status change for customer is mandatory
   t.ticket.cust_status_msg_id = update_status(bot, &mut t, Role::Customer, tag).await?;

   // Update status for owners, ignore fail
   t.ticket.owners_status_msg_id.0 = update_status(bot, &mut t, Role::Owner1, tag)
   .await
   .ok()
   .flatten();

   // The same for owner 2
   t.ticket.owners_status_msg_id.1 = update_status(bot, &mut t, Role::Owner2, tag)
   .await
   .ok()
   .flatten();

   // The same for owner 3
   t.ticket.owners_status_msg_id.2 = update_status(bot, &mut t, Role::Owner3, tag)
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

pub async fn cancel_ticket(bot: &Bot, q: CallbackQuery, ticket_id: i32, tag: LocaleTag) -> Result<String, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = if q.from.id == t.ticket.customer_id {
      Stage::CanceledByCustomer
   } else {
      Stage::CanceledByOwner
   };
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   let stage = t.ticket.stage;
   update_statuses(bot, t, tag).await?;

   // Send the status also to the service chat
   let status = stage.message_for_owner(tag);
   env::log_reply(&status, service_msg_id).await;

   // "Successfully"
   Ok(loc(Key::RegMakeTicket7, tag, &[]))
}

pub async fn next_ticket(bot: &Bot, ticket_id: i32, tag: LocaleTag) -> Result<String, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   if t.ticket.next_stage() {
      // Update status in database if it was really changed
      db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;
   }

   update_statuses(bot, t, tag).await?;
   // "Successfully"
   Ok(loc(Key::RegMakeTicket7, tag, &[]))
}

pub async fn confirm_ticket(bot: &Bot, ticket_id: i32, tag: LocaleTag) -> Result<String, String> {

   // Load ticket and update status
   let mut t = db::ticket_with_owners(ticket_id).await?;
   t.ticket.stage = Stage::Finished;
   db::ticket_update_stage(t.ticket.id, t.ticket.stage).await?;

   let service_msg_id = t.ticket.service_msg_id;
   update_statuses(bot, t, tag).await?;

   // Send the order also to the service chat "Order completed successfully"
   let status = loc(Key::RegConfirmTicket, tag, &[]);
   env::log_reply(&status, service_msg_id).await;

   // "Successfully"
   Ok(loc(Key::RegMakeTicket7, tag, &[]))
}

async fn reply_msg(bot: &Bot, receiver: UserId, reply_to_id: MessageId, text: &str) -> Result<(), String> {
   let mut fut = bot.send_message(receiver, text)
   .parse_mode(ParseMode::Html);

   if reply_to_id.0 != 0 {
      fut = fut.reply_to_message_id(reply_to_id);
   }

   fut.await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(())
}


async fn send_msg(bot: &Bot, receiver: UserId, text: &str) -> ResultMessage {
   let res = bot.send_message(receiver, text)
   .parse_mode(ParseMode::Html)
   .await
   .map_err(|err| format!("registration::::send_msg for receiver={} {}", receiver, err))?;
   Ok(res)
}

async fn forward_msg(bot: &Bot, from: UserId, receiver: UserId, message_id: MessageId) -> ResultMessage {
   let res = bot.forward_message(receiver, from, message_id).await
   .map_err(|err| format!("forward_msg {}", err))?;
   Ok(res)
}

async fn send_msg_to_owners(bot: &Bot, owners: &node::Owners, text: &str) -> Result3Id {
   // Try to send to all owners
   let msg1 = send_msg(bot, owners.0, text).await;
   let msg2 = send_msg(bot, owners.1, text).await;
   let msg3 = send_msg(bot, owners.2, text).await;

   // Report an error from owner1 if there are no successful attempts
   unwrap_msg_id(msg1, msg2, msg3)
}

async fn forward_msg_to_owners(bot: &Bot, from: UserId, owners: &node::Owners, message_id: MessageId) -> Result3Id {
   // Try to send to all owners
   let msg1 = forward_msg(bot, from, owners.0, message_id).await;
   let msg2 = forward_msg(bot, from, owners.1, message_id).await;
   let msg3 = forward_msg(bot, from, owners.2, message_id).await;
   
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
