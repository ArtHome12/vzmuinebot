/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, просмотр корзины. 15 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;

// Показывает список закзов для user_id
//
pub async fn next_with_info(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;
   
   // Получаем информацию из БД
   let (baskets, grand_total) = db::basket_contents(user_id).await;

   if baskets.is_empty() {
      // Отображаем информацию и кнопки меню
      cx.answer("Корзина пуста")
      .reply_markup(cmd::Basket::markup())
         .send()
         .await?;
   } else {
      // Отображаем приветствие
      cx.answer(format!("Общая сумма заказа {} тыс. vnd. Перешлите сообщения ниже по указанным контактам или в независимую доставку:", grand_total))
      .reply_markup(cmd::Basket::markup())
      .send()
      .await?;

      // Отдельными сообщениями выводим рестораны
      for basket in baskets {
         // Заголовок с информацией о ресторане
         let mut s = basket.restaurant;

         // Дополняем данными о блюдах
         for dish in basket.dishes {
            s.push_str(&format!("\n{}", dish))
         }

         // Итоговая стоимость
         s.push_str(&format!("\nВсего: {} тыс. vnd", basket.total));

         cx.answer(s)
         .reply_markup(cmd::Basket::markup())
         .send()
         .await?;
      }
   }

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::BasketMode(user_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<i32>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::Basket::markup())
   .send()
   .await?;

   // Извлечём параметры
   let user_id = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::BasketMode(user_id))
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::Basket::from(command) {

            // В главное меню
            cmd::Basket::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            cmd::Basket::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
            }
         }
      }
   }
}
