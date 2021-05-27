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
   types::{InputFile, KeyboardButton, InlineKeyboardMarkup, 
      InlineKeyboardButton, ReplyMarkup, ButtonRequest, 
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
   // Создадим кнопки под рестораны
   let buttons: Vec<InlineKeyboardButton> = node.children
   .iter()
   .map(|child| (InlineKeyboardButton::callback(child.title.clone(), format!("grc{}", 0))))  // third argument unused
   .collect();

   let (long, mut short) : (Vec<_>, Vec<_>) = buttons
   .into_iter()
   .partition(|n| n.text.chars().count() > 21);

   // Последняя непарная кнопка, если есть
   let last = if short.len() % 2 == 1 { short.pop() } else { None };

   // Сначала длинные кнопки по одной
   let markup = long.into_iter() 
   .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));

   // Короткие по две в ряд
   let markup = short.into_iter().array_chunks::<[_; 2]>()
   .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));
   
   // Возвращаем результат
   if let Some(last_button) = last {
      markup.append_row(vec![last_button])
   } else {
      markup
   }
}
