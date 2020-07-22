/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для хранения prepared-запросов к СУБД. 22 July 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use tokio_postgres::{Statement,};

// Поддерживаемые запросы
pub struct PreparedStatements {
   pub rest_list_all : Statement,
   pub rest_list_category : Statement,
   pub rest_list_time : Statement,
   /*pub restaurant_id : Statement,
   pub restaurant_num : Statement,
   pub group_list_all : Statement,
   pub group_list_category : Statement,
   pub group_list_time : Statement,
   pub group : Statement,
   pub dish_list_all : Statement,
   pub dish_list_active : Statement,
   pub dish_all : Statement,
   pub dish_active : Statement,
   pub user_update_last_seen : Statement,
   pub user_compact_interface : Statement,
   pub amount_in_basket : Statement,
   pub add_dish_to_basket_update : Statement,
   pub add_dish_to_basket_insert : Statement,
   pub remove_dish_from_basket_update : Statement,
   pub remove_dish_from_basket_delete : Statement,
   pub basket_contents : Statement,
   pub basket_content : Statement,
   pub clear_basket : Statement,
   pub eater_ticket_info : Statement,
   pub caterer_ticket_info : Statement,
   pub ticket_with_owners : Statement,
   pub basket_edit_stage : Statement,
   pub basket_next_stage : Statement,
   pub basket_stage : Statement,*/
}

type Client<'a> = bb8::PooledConnection<'a, bb8_postgres::PostgresConnectionManager<tokio_postgres::tls::NoTls>>;

impl PreparedStatements {
   pub async fn from_db<'a>(client: Client<'a>) -> Self {
      PreparedStatements{
         rest_list_all : client.prepare("SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r
               ORDER BY rest_num").await.unwrap(),
         rest_list_category : client.prepare("SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r 
            INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE cat_id=$1::INTEGER AND active = TRUE) g ON r.rest_num = g.rest_num 
            WHERE r.active = TRUE").await.unwrap(),
         rest_list_time : client.prepare("SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r 
            INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE active = TRUE AND 
            ($1::TIME BETWEEN opening_time AND closing_time) OR (opening_time > closing_time AND $1::TIME > opening_time)) g ON r.rest_num = g.rest_num WHERE r.active = TRUE").await.unwrap(),
         /*restaurant_id : Statement,
         restaurant_num : Statement,
         group_list_all : Statement,
         group_list_category : Statement,
         group_list_time : Statement,
         group : Statement,
         dish_list_all : Statement,
         dish_list_active : Statement,
         dish_all : Statement,
         dish_active : Statement,
         user_update_last_seen : Statement,
         user_compact_interface : Statement,
         amount_in_basket : Statement,
         add_dish_to_basket_update : Statement,
         add_dish_to_basket_insert : Statement,
         remove_dish_from_basket_update : Statement,
         remove_dish_from_basket_delete : Statement,
         basket_contents : Statement,
         basket_content : Statement,
         clear_basket : Statement,
         eater_ticket_info : Statement,
         caterer_ticket_info : Statement,
         ticket_with_owners : Statement,
         basket_edit_stage : Statement,
         basket_next_stage : Statement,
         basket_stage : Statement,*/
      }
   }
}