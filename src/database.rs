/* ===============================================================================
Restaurant menu bot.
Database. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use once_cell::sync::{OnceCell};
use deadpool_postgres::{Pool, Client};
use tokio_postgres::types::ToSql;

use crate::environment;
use crate::node::*;

// Пул клиентов БД
pub static DB: OnceCell<Pool> = OnceCell::new();

pub type Params<'a> = &'a[&'a(dyn ToSql + Sync)];

// ============================================================================
// [Nodes table]
// ============================================================================

pub enum LoadNode {
   Owner(i64), // load first node with this owner
   Children(Node), // load children nodes for this
   Id(i32), // load node with specified id
}

pub async fn node(mode: LoadNode) -> Result<Node, String> {
   // DB client from the pool
   let client = db_client().await?;

   // Construct statement from 3 parts: select, where and order
   let select = String::from("SELECT id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price FROM nodes WHERE ");

   let where_tuple = match &mode {
      LoadNode::Children(node) => ("parent = $1::BIGINT", node.id as i64),
      LoadNode::Owner(user_id) =>  ("owner1 = $1::BIGINT OR owner2 = $1::BIGINT OR owner3 = $1::BIGINT", *user_id),
      LoadNode::Id(id) =>  ("id = $1::BIGINT", *id as i64),
   };

   let order = " ORDER BY id";

   let statement_text = select + where_tuple.0 + order;

   // Prepare query
   let statement = client
   .prepare(&statement_text)
   .await
   .map_err(|err| format!("node prepare: {}", err))?;

   // Run query
   let query = client
   .query(&statement, &[&where_tuple.1])
   .await
   .map_err(|err| format!("node query: {}", err))?;

   // Collect results
   match mode {
      
      LoadNode::Children(mut node) => {
         // Clear any old and add new children
         node.children.clear();
         for row in query {
            node.children.push(Node::from(&row))
         }
         
         Ok(node)
      }
      
      LoadNode::Owner(user_id) => {
         // Create new node and initialize it from database
         if query.is_empty() {Err(format!("db::node::LoadNode::Owner Query empty for user_id={}", user_id))}
         else {Ok(Node::from(&query[0]))}
      }

      LoadNode::Id(id) => {
         if query.is_empty() {Err(format!("db::node::LoadNode::Owner Query empty for id={}", id))}
         else {Ok(Node::from(&query[0]))}
      }
   }
}

pub async fn insert_node(node: &Node) -> Result<(), String> {
   // DB client from the pool
   let client = db_client().await?;

   // Information for query
   let statement_text = "INSERT INTO nodes (parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price) \
      VALUES ($1::INTEGER, $2::VARCHAR, $3::VARCHAR, $4::VARCHAR, $5::BOOLEAN, $6::BOOLEAN, $7::BIGINT, $8::BIGINT, $9::BIGINT, $10::TIME, $11::TIME, $12::INTEGER)";

   let params: Params = &[&node.parent,
      &node.title,
      &node.descr,
      &node.picture,
      &node.enabled,
      &node.banned,
      &node.owners[0],
      &node.owners[1],
      &node.owners[2],
      &node.open,
      &node.close,
      &node.price];

   // Prepare query
   let statement = client
   .prepare(&statement_text)
   .await
   .map_err(|err| format!("insert_node prepare: {}", err))?;

   // Run query
   let query = client.execute(&statement, params)
   .await
   .map_err(|err| format!("insert_node execute: {}", err))?;

   // Only one record has to be updated
   if query == 1 { Ok(()) }
   else { Err(format!("insert_node updated {} records", query)) }
}

pub async fn delete_node(id: i32) -> Result<(), String> {
   let client = db_client().await?;

   // Check no children
   let text = "SELECT id FROM nodes WHERE parent = $1::INTEGER";
   let query = client.query(text, &[&id])
   .await
   .map_err(|err| format!("delete_node prepare: {}", err))?;

   let children_num = query.len();
   if children_num > 0 {
      return Err(format!("delete_node has {} children", children_num));
   }

   // Delete node
   let text = "DELETE FROM nodes WHERE id = $1::INTEGER";
   execute_one(text, &[&id]).await
}

pub async fn update_node(id: i32, update: &UpdateNode) -> Result<(), String> {
   match &update.kind {
      UpdateKind::Text(new_val) | UpdateKind::Picture(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::VARCHAR WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Flag(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::BOOLEAN WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
   }
}

// ============================================================================
// [Misc]
// ============================================================================

// Проверяет существование таблиц
pub async fn is_tables_exist() -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_err() {return false;}

   // Выполняем запрос
   let rows = client.unwrap()
   .query("SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_NAME='nodes'", &[]).await;

   // Проверяем результат
   match rows {
      Ok(data) => !data.is_empty(),
      _ => false,
   }
}

// Создаёт новые таблицы
pub async fn create_tables() -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_err() {return false;}

   // Таблица с данными о ресторанах
   let query = client.unwrap()
   .batch_execute("CREATE TABLE nodes (
         PRIMARY KEY (id),
         id             SERIAL         NOT NULL,
         parent         INTEGER        NOT NULL,
         title          VARCHAR        NOT NULL,
         descr          VARCHAR        NOT NULL,
         picture        VARCHAR        NOT NULL,
         enabled        BOOLEAN        NOT NULL,
         banned         BOOLEAN        NOT NULL,
         owner1         BIGINT         NOT NULL,
         owner2         BIGINT         NOT NULL,
         owner3         BIGINT         NOT NULL,
         open           TIME           NOT NULL,
         close          TIME           NOT NULL,
         price          INTEGER        NOT NULL);

         INSERT INTO nodes (id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price)
         VALUES (0, -1, 'Добро пожаловать', '-', '', true, false, 0, 0, 0, '00:00', '00:00', 0);
   ")
   .await;

   match query {
      Ok(_) => true,
      Err(e) => {
         environment::log(&format!("Error create_tables: {}", e)).await;
         false
       }
   }
}

// Успешно-неуспешно
pub fn is_success(flag : bool) -> &'static str {
   if flag {
      "успешно"
  } else {
      "ошибка"
  }
}


// Обёртка, возвращает пул клиентов
async fn db_client() -> Result<Client, String> {
   match DB.get().unwrap().get().await {
      Ok(client) => Ok(client),
      Err(e) => {
         let error = format!("No db client: {}", e);
         environment::log(&error).await;
         Err(error)
      }
   }
}

// Инициализирует структуру с картинками для категорий
pub async fn cat_image_init() {
   // Внесём значения по-умолчанию, а потом попытаемся прочесть их их базы
   /* let mut hash: CatImageList = HashMap::new();
   hash.insert(1, environment::default_photo_id());
   hash.insert(2, environment::default_photo_id());
   hash.insert(3, environment::default_photo_id());
   hash.insert(4, environment::default_photo_id());

   // Получаем клиента БД
   let client = db_client().await;
   if client.is_some() {
      // Выполняем запрос
      let query = client.unwrap()
      .query("SELECT cat_id, image_id FROM category", &[])
      .await;
      match query {
         Ok(rows) => {
            for row in rows {
               let s: Option<String> = row.get(1);
               if s.is_some() {hash.insert(row.get(0), s.unwrap());}
            }
         }
         Err(e) => {
            environment::log(&format!("Error db::cat_image_init: {}", e)).await;
         }
      };
   };

   // Сохраняем данные
   if let Err(_) = CI.set(RwLock::new(hash)) {
      environment::log(&format!("Error db::cat_image_init2")).await;
   } */
}

async fn execute_one(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> Result<(), String> {
   // DB client from the pool
   let client = db_client().await?;

   // Run query
   let query = client.execute(sql_text, params)
   .await
   .map_err(|err| format!("execute_one {} execute: {}", sql_text, err))?;
   
   // Only one records has to be affected
   if query == 1 { Ok(()) }
   else { Err(format!("execute_one {}: affected {} records instead one (params: {:?})", sql_text, query, params)) }
}

