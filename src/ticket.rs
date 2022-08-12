/* ===============================================================================
Restaurant menu bot.
Ticket to placed order. 06 June 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use strum::{AsRefStr, EnumString, EnumMessage};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ChatId};

use crate::callback;
use crate::node;
use crate::general as gen;
use crate::loc::*;

pub type ThreeMsgId = (Option<i32>, Option<i32>, Option<i32>);

#[derive(Copy, Clone)]
#[derive(AsRefStr, EnumString, EnumMessage)]
pub enum Stage {
   // DB value, info for customer, info for owner
   #[strum(to_string = "A", message = "Ожидание подтверждения приёма заказа в работу",
   detailed_message="Подтвердите начало обработки заказа")]
   OwnersConfirmation,

   #[strum(to_string = "B", message = "В процессе приготовления",
   detailed_message="В процессе приготовления. Подтвердите готовность заказа к выдаче")]
   Cooking,

   #[strum(to_string = "C", message = "Готово, идёт доставка",
   detailed_message="В процессе доставки. Подтвердите вручение заказа клиенту")]
   Delivery,

   #[strum(to_string = "D", message = "Подтвердить получение и закрыть заказ",
   detailed_message="Заказ доставлен, ожидание подтверждения со стороны клиента")]
   CustomerConfirmation,

   #[strum(to_string = "X", message = "Завершено", detailed_message="Завершено")]
   Finished,

   #[strum(to_string = "Y", message = "Отменено по инициативе клиента",
   detailed_message="Отменено по инициативе клиента")]
   CanceledByCustomer,

   #[strum(to_string = "Z", message = "Отменено по инициативе заведения",
   detailed_message="Отменено по инициативе заведения")]
   CanceledByOwner,
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
   pub customer_id: i64, // Customer telegram id
   pub cust_msg_id: i32, // Id of the message with order at customer side
   pub owners_msg_id: ThreeMsgId, // The same for owners but any two can be None
   pub stage: Stage, // execution stage
   pub cust_status_msg_id: Option<i32>, // Id of message with execution status at customer side
   pub owners_status_msg_id: ThreeMsgId, // The same for owners but any two can be None
   pub service_msg_id: Option<i32,> // Id of message in service chat
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
   pub fn stage_message(&self, info_for: InfoFor) -> String {
      let (s, id) = match info_for {
         InfoFor::Customer => (self.ticket.stage.get_message().unwrap(), self.owners.0),
         InfoFor::Owner => (self.ticket.stage.get_detailed_message().unwrap(), self.ticket.customer_id),
      };
      format!("{}\nСообщение через бота {}{}", s, gen::Command::Message(ChatId{0:0}).as_ref(), id)
   }
}
