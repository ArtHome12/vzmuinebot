/* ===============================================================================
Restaurant menu bot.
User interface with inline buttons. 27 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use arraylib::iter::IteratorExt;
use teloxide::{
   prelude::*, 
   types::{InputFile, InlineKeyboardButton, InlineKeyboardMarkup, 
      //ReplyMarkup, ButtonRequest, KeyboardButton, 
   },
};

use crate::states::*;
use crate::database as db;
use crate::node::*;


pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Load root node
   let node =  db::node(db::LoadNode::Id(0))
   .await
   .map_err(|s| map_req_err(s))?;

   // Load children
   let node = db::node(db::LoadNode::Children(node)).await
   .map_err(|s| map_req_err(s))?;

   // Picture
   let picture = node.picture.clone();
   if picture.is_none() {
      cx.answer("Ошибка, без картинки невозможно продолжить - обратитесь к администратору")
      .await?;
   } else {
      let picture = picture.unwrap();
      let markup = markup(&node);
      
      cx.answer_photo(InputFile::file_id(picture))
      .caption("Hello")
      .reply_markup(markup)
      .disable_notification(true)
      .send()
      .await?;
   }

   // Always stay in place
   next(state)
}

fn markup(node: &Node) -> InlineKeyboardMarkup {
   // Create buttons for each child
   let buttons: Vec<InlineKeyboardButton> = node.children
   .iter()
   .map(|child| (InlineKeyboardButton::callback(child.title.clone(), format!("grc{}", 0))))  // third argument unused
   .collect();

   // Separate into long and short
   let (long, mut short) : (Vec<_>, Vec<_>) = buttons
   .into_iter()
   .partition(|n| n.text.chars().count() > 21);

   // Put in vec last unpaired button, if any
   let mut last_row = vec![];
   if short.len() % 2 == 1 {
      let unpaired = short.pop();
      if unpaired.is_some() {
         last_row.push(unpaired.unwrap());
      }
   }

   // Long buttons by one in row
   let markup = long.into_iter() 
   .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));

   // Short by two
   let markup = short.into_iter().array_chunks::<[_; 2]>()
   .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));

   // Back button
   if node.id > 0 {
      let button_back = InlineKeyboardButton::callback(String::from("⏪Назад"), format!("rca{}", 0));
      last_row.push(button_back);
   }

   // Add the last unpaired button and the back button
   if last_row.is_empty() {
      markup
   } else {
      markup.append_row(last_row)
   }
}
