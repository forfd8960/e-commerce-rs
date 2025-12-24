use anyhow::Result;
use proto::order::{
    CancelOrderRequest, CancelOrderResponse, CreateOrderRequest, CreateOrderResponse,
    GetOrderRequest, GetOrderResponse, GetOrdersByUserRequest, GetOrdersByUserResponse,
    ListOrdersRequest, ListOrdersResponse, Order, OrderItem, OrderStatus, UpdateOrderRequest,
    UpdateOrderResponse, order_service_server::OrderService,
};
use proto::product;
use proto::product::{CheckAvailabilityRequest, product_service_client::ProductServiceClient};
use proto::user::{VerifyRequest, user_service_client::UserServiceClient};
use sqlx::PgPool;
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct DbOrder {
    id: String,
    user_id: String,
    total_amount: sqlx::types::Decimal,
    status: String,
    shipping_address: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct DbOrderItem {
    id: String,
    order_id: String,
    product_id: String,
    quantity: i32,
    price: sqlx::types::Decimal,
}

pub struct OrderServiceImpl {
    db: PgPool,
    user_service_url: String,
    product_service_url: String,
}

impl OrderServiceImpl {
    pub fn new(db: PgPool, user_service_url: String, product_service_url: String) -> Self {
        Self {
            db,
            user_service_url,
            product_service_url,
        }
    }

    fn status_to_proto(&self, status: &str) -> OrderStatus {
        match status {
            "PENDING" => OrderStatus::Pending,
            "CONFIRMED" => OrderStatus::Confirmed,
            "PROCESSING" => OrderStatus::Processing,
            "SHIPPED" => OrderStatus::Shipped,
            "DELIVERED" => OrderStatus::Delivered,
            "CANCELLED" => OrderStatus::Cancelled,
            _ => OrderStatus::Pending,
        }
    }

    fn status_to_string(&self, status: OrderStatus) -> String {
        match status {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Confirmed => "CONFIRMED",
            OrderStatus::Processing => "PROCESSING",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Delivered => "DELIVERED",
            OrderStatus::Cancelled => "CANCELLED",
        }
        .to_string()
    }

    async fn get_products_by_ids(
        &self,
        product_ids: Vec<String>,
    ) -> Result<std::collections::HashMap<String, product::Product>, Status> {
        let mut product_client = ProductServiceClient::connect(self.product_service_url.clone())
            .await
            .map_err(|e| {
                Status::unavailable(format!("Failed to connect to product service: {}", e))
            })?;

        let product_request = product::GetProductsByIDsRequest {
            product_ids: product_ids.clone(),
        };

        let product_response = product_client
            .get_products_by_ids(product_request)
            .await
            .map_err(|e| Status::internal(format!("Product service error: {}", e)))?;

        let product_result = product_response.into_inner();
        let product_map: std::collections::HashMap<String, product::Product> = product_result
            .products
            .into_iter()
            .map(|p| (p.product_id.clone(), p))
            .collect();

        Ok(product_map)
    }

    async fn get_order_items(&self, order_id: &str) -> Result<Vec<OrderItem>, Status> {
        let db_items = sqlx::query_as::<_, DbOrderItem>(
            "SELECT id, order_id, product_id, quantity, price FROM order_items WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // collect product ids from db_items, and then call product service get_products_by_ids to get products
        let product_ids: Vec<String> = db_items
            .iter()
            .map(|item| item.product_id.clone())
            .collect();

        let product_map = self.get_products_by_ids(product_ids).await?;

        let mut items = Vec::new();
        for db_item in db_items {
            let price = db_item.price.to_string().parse::<f64>().unwrap_or(0.0);
            let subtotal = price * db_item.quantity as f64;

            items.push(OrderItem {
                product_id: db_item.product_id.clone(),
                product_name: product_map
                    .get(&db_item.product_id)
                    .map_or(String::new(), |p| p.name.clone()),
                quantity: db_item.quantity,
                unit_price: price,
                subtotal,
            });
        }

        Ok(items)
    }

    async fn db_order_to_proto(&self, db_order: &DbOrder) -> Result<Order, Status> {
        let items = self.get_order_items(&db_order.id).await?;

        Ok(Order {
            order_id: db_order.id.clone(),
            user_id: db_order.user_id.clone(),
            items,
            total_amount: db_order
                .total_amount
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
            status: self.status_to_proto(&db_order.status) as i32,
            shipping_address: db_order.shipping_address.clone().unwrap_or_default(),
            created_at: db_order.created_at.and_utc().timestamp(),
            updated_at: db_order.updated_at.and_utc().timestamp(),
        })
    }

    async fn verify_user_by_id(&self, user_id: &str) -> Result<bool, Status> {
        // Call user service to verify token and get user_id
        let mut client = UserServiceClient::connect(self.user_service_url.clone())
            .await
            .map_err(|e| {
                Status::unavailable(format!("Failed to connect to user service: {}", e))
            })?;

        let verify_request = VerifyRequest {
            user_id: user_id.to_string(),
        };

        let response = client
            .verify(verify_request)
            .await
            .map_err(|e| Status::internal(format!("User service error: {}", e)))?;

        let result = response.into_inner();

        if result.valid { Ok(true) } else { Ok(false) }
    }

    async fn check_product_availability(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> Result<bool, Status> {
        // Call product service to check availability
        let mut client = ProductServiceClient::connect(self.product_service_url.clone())
            .await
            .map_err(|e| {
                Status::unavailable(format!("Failed to connect to product service: {}", e))
            })?;

        let check_request = CheckAvailabilityRequest {
            product_id: product_id.to_string(),
            quantity,
        };

        let response = client
            .check_availability(check_request)
            .await
            .map_err(|e| Status::internal(format!("Product service error: {}", e)))?;

        let result = response.into_inner();
        Ok(result.available)
    }

    async fn get_product_price(&self, product_id: &str) -> Result<Option<f64>, Status> {
        let price: Option<sqlx::types::Decimal> =
            sqlx::query_scalar("SELECT price FROM products WHERE id = $1")
                .bind(product_id)
                .fetch_optional(&self.db)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(price.map(|p| p.to_string().parse::<f64>().unwrap_or(0.0)))
    }
}

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn create_order(
        &self,
        request: Request<CreateOrderRequest>,
    ) -> Result<Response<CreateOrderResponse>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.user_id.is_empty() {
            return Ok(Response::new(CreateOrderResponse {
                success: false,
                message: "User ID is required".to_string(),
                order_id: String::new(),
                order: None,
            }));
        }

        if req.items.is_empty() {
            return Ok(Response::new(CreateOrderResponse {
                success: false,
                message: "Order must contain at least one item".to_string(),
                order_id: String::new(),
                order: None,
            }));
        }

        // Verify user exists
        if !self.verify_user_by_id(&req.user_id).await? {
            return Ok(Response::new(CreateOrderResponse {
                success: false,
                message: "User not found".to_string(),
                order_id: String::new(),
                order: None,
            }));
        }

        // Check product availability and calculate total
        let mut total_amount = 0.0;
        let mut validated_items = Vec::new();

        for item in &req.items {
            if item.quantity <= 0 {
                return Ok(Response::new(CreateOrderResponse {
                    success: false,
                    message: format!("Invalid quantity for product {}", item.product_id),
                    order_id: String::new(),
                    order: None,
                }));
            }

            // Check availability
            if !self
                .check_product_availability(&item.product_id, item.quantity)
                .await?
            {
                return Ok(Response::new(CreateOrderResponse {
                    success: false,
                    message: format!(
                        "Product {} not available in requested quantity",
                        item.product_id
                    ),
                    order_id: String::new(),
                    order: None,
                }));
            }

            // Get current price
            let price = match self.get_product_price(&item.product_id).await? {
                Some(p) => p,
                None => {
                    return Ok(Response::new(CreateOrderResponse {
                        success: false,
                        message: format!("Product {} not found", item.product_id),
                        order_id: String::new(),
                        order: None,
                    }));
                }
            };

            let subtotal = price * item.quantity as f64;
            total_amount += subtotal;

            validated_items.push((item, price));
        }

        // Start transaction
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|e| Status::internal(format!("Transaction error: {}", e)))?;

        let order_id = Uuid::new_v4().to_string();
        let total_decimal = sqlx::types::Decimal::from_f64_retain(total_amount)
            .ok_or_else(|| Status::invalid_argument("Invalid total amount"))?;

        // Create order
        sqlx::query(
            "INSERT INTO orders (id, user_id, total_amount, status, shipping_address) 
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&order_id)
        .bind(&req.user_id)
        .bind(total_decimal)
        .bind("PENDING")
        .bind(if req.shipping_address.is_empty() {
            None
        } else {
            Some(&req.shipping_address)
        })
        .execute(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // Create order items and update inventory
        for (item, price) in validated_items {
            let item_id = Uuid::new_v4().to_string();
            let price_decimal = sqlx::types::Decimal::from_f64_retain(price)
                .ok_or_else(|| Status::invalid_argument("Invalid price"))?;

            sqlx::query(
                "INSERT INTO order_items (id, order_id, product_id, quantity, price) 
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&item_id)
            .bind(&order_id)
            .bind(&item.product_id)
            .bind(item.quantity)
            .bind(price_decimal)
            .execute(&mut *tx)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            // Update product inventory
            sqlx::query(
                "UPDATE products SET stock_quantity = stock_quantity - $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
            )
            .bind(item.quantity)
            .bind(&item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| Status::internal(format!("Commit error: {}", e)))?;

        // Fetch created order
        let order = sqlx::query_as::<_, DbOrder>(
            "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
             FROM orders WHERE id = $1",
        )
        .bind(&order_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let proto_order = self.db_order_to_proto(&order).await?;

        Ok(Response::new(CreateOrderResponse {
            success: true,
            message: "Order created successfully".to_string(),
            order_id,
            order: Some(proto_order),
        }))
    }

    async fn update_order(
        &self,
        request: Request<UpdateOrderRequest>,
    ) -> Result<Response<UpdateOrderResponse>, Status> {
        let req = request.into_inner();

        if req.order_id.is_empty() {
            return Ok(Response::new(UpdateOrderResponse {
                success: false,
                message: "Order ID is required".to_string(),
                order: None,
            }));
        }

        let status_str = self
            .status_to_string(OrderStatus::try_from(req.status).unwrap_or(OrderStatus::Pending));

        let result = sqlx::query(
            "UPDATE orders SET status = $1, shipping_address = $2, updated_at = CURRENT_TIMESTAMP 
             WHERE id = $3",
        )
        .bind(&status_str)
        .bind(if req.shipping_address.is_empty() {
            None
        } else {
            Some(&req.shipping_address)
        })
        .bind(&req.order_id)
        .execute(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Ok(Response::new(UpdateOrderResponse {
                success: false,
                message: "Order not found".to_string(),
                order: None,
            }));
        }

        // Fetch updated order
        let order = sqlx::query_as::<_, DbOrder>(
            "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
             FROM orders WHERE id = $1",
        )
        .bind(&req.order_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let proto_order = self.db_order_to_proto(&order).await?;

        Ok(Response::new(UpdateOrderResponse {
            success: true,
            message: "Order updated successfully".to_string(),
            order: Some(proto_order),
        }))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let req = request.into_inner();

        if req.order_id.is_empty() {
            return Ok(Response::new(CancelOrderResponse {
                success: false,
                message: "Order ID is required".to_string(),
            }));
        }

        // Start transaction to restore inventory
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|e| Status::internal(format!("Transaction error: {}", e)))?;

        // Check if order exists and belongs to user
        let order: Option<DbOrder> = sqlx::query_as(
            "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
             FROM orders WHERE id = $1",
        )
        .bind(&req.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let order = match order {
            Some(o) => o,
            None => {
                tx.rollback()
                    .await
                    .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
                return Ok(Response::new(CancelOrderResponse {
                    success: false,
                    message: "Order not found".to_string(),
                }));
            }
        };

        if !req.user_id.is_empty() && order.user_id != req.user_id {
            tx.rollback()
                .await
                .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
            return Ok(Response::new(CancelOrderResponse {
                success: false,
                message: "Order does not belong to this user".to_string(),
            }));
        }

        if order.status == "CANCELLED" {
            tx.rollback()
                .await
                .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
            return Ok(Response::new(CancelOrderResponse {
                success: false,
                message: "Order is already cancelled".to_string(),
            }));
        }

        if order.status == "DELIVERED" {
            tx.rollback()
                .await
                .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
            return Ok(Response::new(CancelOrderResponse {
                success: false,
                message: "Cannot cancel delivered order".to_string(),
            }));
        }

        // Restore inventory
        let items = sqlx::query_as::<_, DbOrderItem>(
            "SELECT id, order_id, product_id, quantity, price FROM order_items WHERE order_id = $1",
        )
        .bind(&req.order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        for item in items {
            sqlx::query(
                "UPDATE products SET stock_quantity = stock_quantity + $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
            )
            .bind(item.quantity)
            .bind(&item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        }

        // Update order status
        sqlx::query(
            "UPDATE orders SET status = 'CANCELLED', updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(&req.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| Status::internal(format!("Commit error: {}", e)))?;

        Ok(Response::new(CancelOrderResponse {
            success: true,
            message: "Order cancelled successfully".to_string(),
        }))
    }

    async fn get_order(
        &self,
        request: Request<GetOrderRequest>,
    ) -> Result<Response<GetOrderResponse>, Status> {
        let req = request.into_inner();

        if req.order_id.is_empty() {
            return Ok(Response::new(GetOrderResponse {
                success: false,
                message: "Order ID is required".to_string(),
                order: None,
            }));
        }

        let order_result = sqlx::query_as::<_, DbOrder>(
            "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
             FROM orders WHERE id = $1",
        )
        .bind(&req.order_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match order_result {
            Some(order) => {
                let proto_order = self.db_order_to_proto(&order).await?;
                Ok(Response::new(GetOrderResponse {
                    success: true,
                    message: "Order retrieved successfully".to_string(),
                    order: Some(proto_order),
                }))
            }
            None => Ok(Response::new(GetOrderResponse {
                success: false,
                message: "Order not found".to_string(),
                order: None,
            })),
        }
    }

    async fn list_orders(
        &self,
        request: Request<ListOrdersRequest>,
    ) -> Result<Response<ListOrdersResponse>, Status> {
        let req = request.into_inner();

        let page = if req.page <= 0 { 1 } else { req.page };
        let page_size = if req.page_size <= 0 || req.page_size > 100 {
            10
        } else {
            req.page_size
        };
        let offset = (page - 1) * page_size;

        let status = OrderStatus::try_from(req.status).unwrap_or(OrderStatus::Pending);
        let status_str = self.status_to_string(status);

        let (orders, total_count) = if req.status == 0 {
            // List all orders
            let orders = sqlx::query_as::<_, DbOrder>(
                "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
                 FROM orders 
                 ORDER BY created_at DESC 
                 LIMIT $1 OFFSET $2",
            )
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
                .fetch_one(&self.db)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            (orders, count.0)
        } else {
            // Filter by status
            let orders = sqlx::query_as::<_, DbOrder>(
                "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
                 FROM orders 
                 WHERE status = $1 
                 ORDER BY created_at DESC 
                 LIMIT $2 OFFSET $3",
            )
            .bind(&status_str)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = $1")
                .bind(&status_str)
                .fetch_one(&self.db)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            (orders, count.0)
        };

        let mut proto_orders = Vec::new();
        for order in orders {
            proto_orders.push(self.db_order_to_proto(&order).await?);
        }

        Ok(Response::new(ListOrdersResponse {
            success: true,
            message: format!("Retrieved {} orders", proto_orders.len()),
            orders: proto_orders,
            total_count: total_count as i32,
        }))
    }

    async fn get_orders_by_user(
        &self,
        request: Request<GetOrdersByUserRequest>,
    ) -> Result<Response<GetOrdersByUserResponse>, Status> {
        let req = request.into_inner();

        if req.user_id.is_empty() {
            return Ok(Response::new(GetOrdersByUserResponse {
                success: false,
                message: "User ID is required".to_string(),
                orders: vec![],
                total_count: 0,
            }));
        }

        let page = if req.page <= 0 { 1 } else { req.page };
        let page_size = if req.page_size <= 0 || req.page_size > 100 {
            10
        } else {
            req.page_size
        };
        let offset = (page - 1) * page_size;

        let orders = sqlx::query_as::<_, DbOrder>(
            "SELECT id, user_id, total_amount, status, shipping_address, created_at, updated_at 
             FROM orders 
             WHERE user_id = $1 
             ORDER BY created_at DESC 
             LIMIT $2 OFFSET $3",
        )
        .bind(&req.user_id)
        .bind(page_size as i64)
        .bind(offset as i64)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE user_id = $1")
            .bind(&req.user_id)
            .fetch_one(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let mut proto_orders = Vec::new();
        for order in orders {
            proto_orders.push(self.db_order_to_proto(&order).await?);
        }

        Ok(Response::new(GetOrdersByUserResponse {
            success: true,
            message: format!("Retrieved {} orders for user", proto_orders.len()),
            orders: proto_orders,
            total_count: count.0 as i32,
        }))
    }
}
