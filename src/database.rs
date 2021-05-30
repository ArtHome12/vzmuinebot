/* ===============================================================================
Restaurant menu bot.
Database. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use once_cell::sync::{OnceCell};
use deadpool_postgres::{Pool, Client, };
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::{types::ToSql, Row, };
use async_recursion::async_recursion;

use crate::environment as env;
use crate::node::*;

// Пул клиентов БД
pub type PoolAlias = Pool<MakeTlsConnector>;
pub static DB: OnceCell<PoolAlias> = OnceCell::new();

pub type Params<'a> = &'a[&'a(dyn ToSql + Sync)];

// ============================================================================
// [Nodes table]
// ============================================================================

pub enum LoadNode {
   Owner(i64), // load first node with this owner
   Id(i32), // load node with specified id
   EnabledId(i32), // like Id but without disabled
   EnabledNowId(i32), // like EnabledId but opened now
   Children(Node), // load children nodes for this
   EnabledChildren(Node), // like Children but without disabled
   EnabledChildrenNow(Node), // like EnabledChildren but opened now
}

#[async_recursion]
pub async fn node(mode: LoadNode) -> Result<Option<Node>, String> {
   async fn do_load_node(rows: Vec<Row>) -> Result<Option<Node>, String> {
      if rows.is_empty() {Ok(None)}
      else {
         // Create node
         let mut start_node =  Node::from(&rows[0]);

         // Try to find picture if not
         if start_node.picture.is_none() {
            start_node.picture = lookup_picture(start_node.parent).await?;
         }

         Ok(Some(start_node))
      }
   }

   async fn do_load_children(mode: LoadNode) -> Result<Option<Node>, String> {
      // Recursively load its children
      let with_children = node(mode)
      .await?
      .unwrap();
      Ok(Some(with_children))
   }

   // DB client from the pool
   let client = db_client().await?;

   // Construct statement from 3 parts: select, where and order
   let select = String::from("SELECT id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price FROM nodes WHERE ");

   let where_tuple = match &mode {
      LoadNode::Owner(user_id) =>  ("owner1 = $1::BIGINT OR owner2 = $1::BIGINT OR owner3 = $1::BIGINT", *user_id),
      LoadNode::Id(id) =>  ("id = $1::BIGINT", *id as i64),
      LoadNode::EnabledId(id) =>  ("id = $1::BIGINT AND enabled = TRUE AND banned = FALSE", *id as i64),
      LoadNode::EnabledNowId(id) => (
         "id = $1::BIGINT AND enabled = TRUE AND banned = FALSE AND
         (($2::TIME BETWEEN open AND close) OR (open >= close AND $2::TIME > open))", *id as i64
      ),
      LoadNode::Children(node) => ("parent = $1::BIGINT", node.id as i64),
      LoadNode::EnabledChildren(node) => ("parent = $1::BIGINT AND enabled = TRUE AND banned = FALSE", node.id as i64),
      LoadNode::EnabledChildrenNow(node) => (
         "parent = $1::BIGINT AND enabled = TRUE AND banned = FALSE AND
         (($2::TIME BETWEEN open AND close) OR (open >= close AND $2::TIME > open))", node.id as i64
      ),
   };

   let order = " ORDER BY id";

   let statement_text = select + where_tuple.0 + order;

   // Prepare query
   let statement = client
   .prepare(&statement_text)
   .await
   .map_err(|err| format!("node prepare: {}", err))?;

   // Run query
   let query = match &mode {
      LoadNode::EnabledNowId(_)
      | &LoadNode::EnabledChildrenNow(_) => {
         // Current local time
         let time = env::current_date_time().time();
         client.query(&statement, &[&where_tuple.1, &time]).await
      }
      _ => client.query(&statement, &[&where_tuple.1]).await
   }
   .map_err(|err| format!("node query: {}", err))?;

   // Collect results
   match mode {

      LoadNode::Children(mut node)
      | LoadNode::EnabledChildren(mut node)
      | LoadNode::EnabledChildrenNow(mut node) => {
         // Clear any old and add new children
         node.children.clear();
         for row in query {
            // Create and inherit the picture if none
            let mut child = Node::from(&row);
            if child.picture.is_none() {
               child.picture = node.picture.clone();
            }

            node.children.push(child);
         }

         Ok(Some(node))
      }

      LoadNode::Owner(_)
      | LoadNode::Id(_) => {
         // Create new node and initialize it from database
         let mut res = do_load_node(query).await?;
         if res.is_some() {
            res = do_load_children(LoadNode::Children(res.unwrap())).await?;
         }
         Ok(res)
      }
      LoadNode::EnabledId(_) => {
         // Create new node and initialize it from database
         let mut res = do_load_node(query).await?;
         if res.is_some() {
            res = do_load_children(LoadNode::EnabledChildren(res.unwrap())).await?;
         }
         Ok(res)
      }
      LoadNode::EnabledNowId(_) => {
         // Create new node and initialize it from database
         let mut res = do_load_node(query).await?;
         if res.is_some() {
            res = do_load_children(LoadNode::EnabledChildrenNow(res.unwrap())).await?;
         }
         Ok(res)
      }
   }
}

async fn lookup_picture(node_id: i32) -> Result<Option<String>, String> {
   let query = "WITH RECURSIVE cte AS (
         SELECT id, parent, picture FROM nodes WHERE id = $1::INTEGER
         UNION SELECT n.id, n.parent, n.picture FROM nodes n
         INNER JOIN cte ON cte.parent = n.id
      ) SELECT picture FROM cte WHERE picture IS NOT NULL LIMIT 1";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("lookup_picture prepare: {}", err))?;

   // Run query
   let query = client
   .query(&statement, &[&node_id])
   .await
   .map_err(|err| format!("lookup_picture query: {}", err))?;

   // Collect result
   let res = query.last()
   .map(|row| row.get(0));

   Ok(res)
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
      &node.time.0,
      &node.time.1,
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
      UpdateKind::Text(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::VARCHAR WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Picture(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::VARCHAR WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Flag(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::BOOLEAN WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Int(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::BIGINT WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Time(open, close) => {
         let text = "UPDATE nodes SET open = $1::TIME, close = $2::TIME WHERE id=$3::INTEGER";
         execute_one(text, &[open, close, &id]).await
      }
      UpdateKind::Money(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::INTEGER WHERE id=$2::INTEGER", update.field);
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
         picture        VARCHAR,
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
         env::log(&format!("Error create_tables: {}", e)).await;
         false
       }
   }
}

// Convert bool to text
pub fn is_success(flag : bool) -> &'static str {
   if flag {
      "успешно"
  } else {
      "ошибка"
  }
}

// Обёртка, возвращает пул клиентов
async fn db_client() -> Result<Client::<MakeTlsConnector>, String> {
   match DB.get().unwrap().get().await {
      Ok(client) => Ok(client),
      Err(e) => {
         let error = format!("No db client: {}", e);
         env::log(&error).await;
         Err(error)
      }
   }
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

