/* ===============================================================================
Restaurant menu bot.
Ticket to placed order. 06 June 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use strum::{AsRefStr, EnumString};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ChatId, MessageId,
   UserId,
};

use crate::callback;
use crate::node;
use crate::general as gen;
use crate::loc::*;

pub type ThreeMsgId = (Option<MessageId>, Option<MessageId>, Option<MessageId>);

pub fn three_msg_id_to_int(three_msg_id: &ThreeMsgId) -> (Option<i32>, Option<i32>, Option<i32>) {
   (
      three_msg_id.0.map(|id| id.0),
      three_msg_id.1.map(|id| id.0),
      three_msg_id.2.map(|id| id.0),
   )
}

pub fn option_to_msg_id(opt: Option<i32>) -> Option<MessageId> {
   opt.and_then(|val| Some(MessageId(val)))
}

pub fn three_option_to_msg_id(opt1: Option<i32>, opt2: Option<i32>, opt3: Option<i32>) -> ThreeMsgId {
   (option_to_msg_id(opt1), option_to_msg_id(opt2), option_to_msg_id(opt3))
}

#[derive(Copy, Clone)]
#[derive(AsRefStr, EnumString)]
pub enum Stage {
   // DB value, info for customer, info for owner
   #[strum(to_string = "A")]
   OwnersConfirmation,

   #[strum(to_string = "B")]
   Cooking,

   #[strum(to_string = "C")]
   Delivery,

   #[strum(to_string = "D")]
   CustomerConfirmation,

   #[strum(to_string = "X")]
   Finished,

   #[strum(to_string = "Y")]
   CanceledByCustomer,

   #[strum(to_string = "Z")]
   CanceledByOwner,
}

impl Stage {
   pub fn message_for_owner(&self, tag: LocaleTag) -> String {
      match self {
         // Waiting for confirmation of acceptance of the order for work
         Stage::OwnersConfirmation => loc(Key::TicketOwner1, tag, &[]),
         // In progress
         Stage::Cooking => loc(Key::TicketOwner2, tag, &[]),
         // Done, delivery in progress
         Stage::Delivery => loc(Key::TicketOwner3, tag, &[]),
         // Confirm receipt and close order
         Stage::CustomerConfirmation => loc(Key::TicketOwner4, tag, &[]),
         // Completed
         Stage::Finished => loc(Key::TicketOwner5, tag, &[]),
         // Canceled by customer
         Stage::CanceledByCustomer => loc(Key::TicketOwner6, tag, &[]),
         // Canceled at the place's initiative
         Stage::CanceledByOwner => loc(Key::TicketOwner7, tag, &[]),
      }
   }

   pub fn message_for_customer(&self, tag: LocaleTag) -> String {
      match self {
         // Confirm the start of order processing
         Stage::OwnersConfirmation => loc(Key::TicketCustomer1, tag, &[]),
         // In progress. Confirm that the order is ready for pickup
         Stage::Cooking => loc(Key::TicketCustomer2, tag, &[]),
         // In the process of delivery. Confirm delivery of the order to the customer
         Stage::Delivery => loc(Key::TicketCustomer3, tag, &[]),
         // Order delivered, waiting for customer confirmation
         Stage::CustomerConfirmation => loc(Key::TicketCustomer4, tag, &[]),
         // Completed
         Stage::Finished => loc(Key::TicketCustomer5, tag, &[]),
         // Canceled by customer
         Stage::CanceledByCustomer => loc(Key::TicketCustomer6, tag, &[]),
         // Canceled at the place's initiative
         Stage::CanceledByOwner => loc(Key::TicketCustomer7, tag, &[]),
      }
   }
}

// The message to the customer and owner is different in markup
#[derive(Copy, Clone)]
pub enum InfoFor {
   Customer,
   Owner,
}

pub struct Ticket {
   pub id: i32, // DB primary key
   pub node_id: i32, // Id of node with owners
   pub customer_id: UserId, // Customer telegram id
   pub cust_msg_id: MessageId, // Id of the message with order at customer side
   pub owners_msg_id: ThreeMsgId, // The same for owners but any two can be None
   pub stage: Stage, // execution stage
   pub cust_status_msg_id: Option<MessageId>, // Id of message with execution status at customer side
   pub owners_status_msg_id: ThreeMsgId, // The same for owners but any two can be None
   pub service_msg_id: Option<MessageId,> // Id of message in service chat
}

pub struct TicketWithOwners {
   pub ticket: Ticket,
   pub owners: node::Owners,
}

impl Ticket {
   pub fn make_markup(&self, info_for: InfoFor, tag: LocaleTag) -> Option<InlineKeyboardMarkup> {
      match info_for {
         InfoFor::Customer => {
            match self.stage {
               Stage::OwnersConfirmation
               | Stage::Cooking
               | Stage::Delivery => Some(self.markup_cancel(tag)),
               Stage::CustomerConfirmation => Some(self.markup_confirm(tag)),
               _ => None,
            }
         }
         InfoFor::Owner => {
            match self.stage {
               Stage::OwnersConfirmation
               | Stage::Cooking
               | Stage::Delivery => Some(self.markup_next(tag)),
               Stage::CustomerConfirmation => Some(self.markup_cancel(tag)),
               _ => None,
            }
         }
      }
   }

   fn button(&self, cmd: callback::Command, tag: LocaleTag) -> InlineKeyboardButton {
      let title = cmd.buttton_caption(tag);
      let args = format!("{}{}", cmd.as_ref(), self.id);
      InlineKeyboardButton::callback(title, args)
   }

   // Menu to cancel ticket at middle
   fn markup_cancel(&self, tag: LocaleTag) -> InlineKeyboardMarkup {
      let cmd = callback::Command::TicketCancel(0);
   
      InlineKeyboardMarkup::default()
      .append_row(vec![self.button(cmd, tag)])
   }

   // Menu for customer to finish ticket
   pub fn markup_confirm(&self, tag: LocaleTag) -> InlineKeyboardMarkup {
      let cancel = callback::Command::TicketCancel(0);
      let confirm = callback::Command::TicketConfirm(0);

      InlineKeyboardMarkup::default()
      .append_row(vec![self.button(cancel, tag), self.button(confirm, tag)])
   }

   // Menu for owner to process ticket
   pub fn markup_next(&self, tag: LocaleTag) -> InlineKeyboardMarkup {
      let cancel = callback::Command::TicketCancel(0);
      let next = callback::Command::TicketNext(0);

      InlineKeyboardMarkup::default()
      .append_row(vec![self.button(cancel, tag), self.button(next, tag)])
   }

   // Go to the next stage if it possible
   pub fn next_stage(&mut self) -> bool {
      self.stage = match self.stage {
         Stage::OwnersConfirmation => Stage::Cooking,
         Stage::Cooking => Stage::Delivery,
         Stage::Delivery => Stage::CustomerConfirmation,
         _ => return false,
      };
      true
   }
}

impl TicketWithOwners {
   pub fn stage_message(&self, info_for: InfoFor, tag: LocaleTag) -> String {
      let (s, id) = match info_for {
         InfoFor::Customer => (self.ticket.stage.message_for_customer(tag), self.owners.0),
         InfoFor::Owner => (self.ticket.stage.message_for_owner(tag), self.ticket.customer_id),
      };
      let cmd = gen::Command::Message(ChatId(0)).as_ref();
      loc(Key::TicketMessage, tag, &[
         &s,
         &cmd,
         &id
      ])
   }
}
