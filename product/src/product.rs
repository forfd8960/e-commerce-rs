use anyhow::Result;
use proto::product::{
    AddProductRequest, AddProductResponse, CheckAvailabilityRequest, CheckAvailabilityResponse,
    DeleteProductRequest, DeleteProductResponse, GetProductRequest, GetProductResponse,
    GetProductsByIDsRequest, GetProductsByIDsResponse, ListProductsRequest, ListProductsResponse,
    Product, UpdateInventoryRequest, UpdateInventoryResponse, UpdateProductRequest,
    UpdateProductResponse, product_service_server::ProductService,
};
use sqlx::{PgPool, types::Decimal};
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct DbProduct {
    id: String,
    name: String,
    description: Option<String>,
    price: sqlx::types::Decimal,
    stock_quantity: i32,
    category: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

pub struct ProductServiceImpl {
    db: PgPool,
}

impl ProductServiceImpl {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    fn db_product_to_proto(&self, db_product: &DbProduct) -> Product {
        Product {
            product_id: db_product.id.clone(),
            name: db_product.name.clone(),
            description: db_product.description.clone().unwrap_or_default(),
            price: db_product.price.to_string().parse::<f64>().unwrap_or(0.0),
            stock_quantity: db_product.stock_quantity,
            category: db_product.category.clone().unwrap_or_default(),
            created_at: db_product.created_at.and_utc().timestamp(),
            updated_at: db_product.updated_at.and_utc().timestamp(),
        }
    }
}

#[tonic::async_trait]
impl ProductService for ProductServiceImpl {
    async fn add_product(
        &self,
        request: Request<AddProductRequest>,
    ) -> Result<Response<AddProductResponse>, Status> {
        let req = request.into_inner();

        // Validate input
        if req.name.is_empty() {
            return Ok(Response::new(AddProductResponse {
                success: false,
                message: "Product name is required".to_string(),
                product_id: String::new(),
            }));
        }

        if req.price < 0.0 {
            return Ok(Response::new(AddProductResponse {
                success: false,
                message: "Price cannot be negative".to_string(),
                product_id: String::new(),
            }));
        }

        if req.stock_quantity < 0 {
            return Ok(Response::new(AddProductResponse {
                success: false,
                message: "Stock quantity cannot be negative".to_string(),
                product_id: String::new(),
            }));
        }

        let product_id = Uuid::new_v4().to_string();
        let price_decimal = Decimal::from_f64_retain(req.price)
            .ok_or_else(|| Status::invalid_argument("Invalid price value"))?;

        // Insert product into database
        let result = sqlx::query(
            "INSERT INTO products (id, name, description, price, stock_quantity, category) 
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&product_id)
        .bind(&req.name)
        .bind(if req.description.is_empty() {
            None
        } else {
            Some(&req.description)
        })
        .bind(price_decimal)
        .bind(req.stock_quantity)
        .bind(if req.category.is_empty() {
            None
        } else {
            Some(&req.category)
        })
        .execute(&self.db)
        .await;

        match result {
            Ok(_) => Ok(Response::new(AddProductResponse {
                success: true,
                message: "Product added successfully".to_string(),
                product_id,
            })),
            Err(e) => Err(Status::internal(format!("Database error: {}", e))),
        }
    }

    async fn update_product(
        &self,
        request: Request<UpdateProductRequest>,
    ) -> Result<Response<UpdateProductResponse>, Status> {
        let req = request.into_inner();

        if req.product_id.is_empty() {
            return Ok(Response::new(UpdateProductResponse {
                success: false,
                message: "Product ID is required".to_string(),
                product: None,
            }));
        }

        if req.price < 0.0 {
            return Ok(Response::new(UpdateProductResponse {
                success: false,
                message: "Price cannot be negative".to_string(),
                product: None,
            }));
        }

        if req.stock_quantity < 0 {
            return Ok(Response::new(UpdateProductResponse {
                success: false,
                message: "Stock quantity cannot be negative".to_string(),
                product: None,
            }));
        }

        let price_decimal = Decimal::from_f64_retain(req.price)
            .ok_or_else(|| Status::invalid_argument("Invalid price value"))?;

        // Update product in database
        let result = sqlx::query(
            "UPDATE products 
             SET name = $1, description = $2, price = $3, stock_quantity = $4, 
                 category = $5, updated_at = CURRENT_TIMESTAMP 
             WHERE id = $6",
        )
        .bind(&req.name)
        .bind(if req.description.is_empty() {
            None
        } else {
            Some(&req.description)
        })
        .bind(price_decimal)
        .bind(req.stock_quantity)
        .bind(if req.category.is_empty() {
            None
        } else {
            Some(&req.category)
        })
        .bind(&req.product_id)
        .execute(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Ok(Response::new(UpdateProductResponse {
                success: false,
                message: "Product not found".to_string(),
                product: None,
            }));
        }

        // Fetch updated product
        let product = sqlx::query_as::<_, DbProduct>(
            "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
             FROM products WHERE id = $1",
        )
        .bind(&req.product_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(UpdateProductResponse {
            success: true,
            message: "Product updated successfully".to_string(),
            product: Some(self.db_product_to_proto(&product)),
        }))
    }

    async fn delete_product(
        &self,
        request: Request<DeleteProductRequest>,
    ) -> Result<Response<DeleteProductResponse>, Status> {
        let req = request.into_inner();

        if req.product_id.is_empty() {
            return Ok(Response::new(DeleteProductResponse {
                success: false,
                message: "Product ID is required".to_string(),
            }));
        }

        let result = sqlx::query("DELETE FROM products WHERE id = $1")
            .bind(&req.product_id)
            .execute(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Ok(Response::new(DeleteProductResponse {
                success: false,
                message: "Product not found".to_string(),
            }));
        }

        Ok(Response::new(DeleteProductResponse {
            success: true,
            message: "Product deleted successfully".to_string(),
        }))
    }

    async fn get_product(
        &self,
        request: Request<GetProductRequest>,
    ) -> Result<Response<GetProductResponse>, Status> {
        let req = request.into_inner();

        if req.product_id.is_empty() {
            return Ok(Response::new(GetProductResponse {
                success: false,
                message: "Product ID is required".to_string(),
                product: None,
            }));
        }

        let product_result = sqlx::query_as::<_, DbProduct>(
            "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
             FROM products WHERE id = $1",
        )
        .bind(&req.product_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match product_result {
            Some(product) => Ok(Response::new(GetProductResponse {
                success: true,
                message: "Product retrieved successfully".to_string(),
                product: Some(self.db_product_to_proto(&product)),
            })),
            None => Ok(Response::new(GetProductResponse {
                success: false,
                message: "Product not found".to_string(),
                product: None,
            })),
        }
    }

    async fn get_products_by_ids(
        &self,
        request: Request<GetProductsByIDsRequest>,
    ) -> Result<Response<GetProductsByIDsResponse>, Status> {
        let req = request.into_inner();

        if req.product_ids.is_empty() {
            return Ok(Response::new(GetProductsByIDsResponse { products: vec![] }));
        }

        let products = sqlx::query_as::<_, DbProduct>(
            "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
             FROM products WHERE id = ANY($1)",
        )
        .bind(&req.product_ids)
        .fetch_all(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let proto_products: Vec<Product> = products
            .iter()
            .map(|p| self.db_product_to_proto(p))
            .collect();

        Ok(Response::new(GetProductsByIDsResponse {
            products: proto_products,
        }))
    }

    async fn list_products(
        &self,
        request: Request<ListProductsRequest>,
    ) -> Result<Response<ListProductsResponse>, Status> {
        let req = request.into_inner();

        let page = if req.page <= 0 { 1 } else { req.page };
        let page_size = if req.page_size <= 0 || req.page_size > 100 {
            10
        } else {
            req.page_size
        };
        let offset = (page - 1) * page_size;

        // Build query based on category filter
        let (products, total_count) = if req.category.is_empty() {
            let products = sqlx::query_as::<_, DbProduct>(
                "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
                 FROM products 
                 ORDER BY created_at DESC 
                 LIMIT $1 OFFSET $2",
            )
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products")
                .fetch_one(&self.db)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            (products, count.0)
        } else {
            let products = sqlx::query_as::<_, DbProduct>(
                "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
                 FROM products 
                 WHERE category = $1 
                 ORDER BY created_at DESC 
                 LIMIT $2 OFFSET $3",
            )
            .bind(&req.category)
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE category = $1")
                .bind(&req.category)
                .fetch_one(&self.db)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            (products, count.0)
        };

        let proto_products: Vec<Product> = products
            .iter()
            .map(|p| self.db_product_to_proto(p))
            .collect();

        Ok(Response::new(ListProductsResponse {
            success: true,
            message: format!("Retrieved {} products", proto_products.len()),
            products: proto_products,
            total_count: total_count as i32,
        }))
    }

    async fn check_availability(
        &self,
        request: Request<CheckAvailabilityRequest>,
    ) -> Result<Response<CheckAvailabilityResponse>, Status> {
        let req = request.into_inner();

        if req.product_id.is_empty() {
            return Ok(Response::new(CheckAvailabilityResponse {
                available: false,
                message: "Product ID is required".to_string(),
                current_stock: 0,
            }));
        }

        let product_result = sqlx::query_as::<_, DbProduct>(
            "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
             FROM products WHERE id = $1",
        )
        .bind(&req.product_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match product_result {
            Some(product) => {
                let available = product.stock_quantity >= req.quantity;
                Ok(Response::new(CheckAvailabilityResponse {
                    available,
                    message: if available {
                        "Product is available".to_string()
                    } else {
                        format!(
                            "Insufficient stock. Available: {}, Requested: {}",
                            product.stock_quantity, req.quantity
                        )
                    },
                    current_stock: product.stock_quantity,
                }))
            }
            None => Ok(Response::new(CheckAvailabilityResponse {
                available: false,
                message: "Product not found".to_string(),
                current_stock: 0,
            })),
        }
    }

    async fn update_inventory(
        &self,
        request: Request<UpdateInventoryRequest>,
    ) -> Result<Response<UpdateInventoryResponse>, Status> {
        let req = request.into_inner();

        if req.product_id.is_empty() {
            return Ok(Response::new(UpdateInventoryResponse {
                success: false,
                message: "Product ID is required".to_string(),
                new_stock_quantity: 0,
            }));
        }

        // Use transaction to ensure atomic update
        let mut tx = self
            .db
            .begin()
            .await
            .map_err(|e| Status::internal(format!("Transaction error: {}", e)))?;

        // Get current stock
        let product_result = sqlx::query_as::<_, DbProduct>(
            "SELECT id, name, description, price, stock_quantity, category, created_at, updated_at 
             FROM products WHERE id = $1 FOR UPDATE",
        )
        .bind(&req.product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let product = match product_result {
            Some(p) => p,
            None => {
                tx.rollback()
                    .await
                    .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
                return Ok(Response::new(UpdateInventoryResponse {
                    success: false,
                    message: "Product not found".to_string(),
                    new_stock_quantity: 0,
                }));
            }
        };

        let new_stock = product.stock_quantity + req.quantity_change;

        if new_stock < 0 {
            tx.rollback()
                .await
                .map_err(|e| Status::internal(format!("Rollback error: {}", e)))?;
            return Ok(Response::new(UpdateInventoryResponse {
                success: false,
                message: format!(
                    "Insufficient stock. Current: {}, Change: {}",
                    product.stock_quantity, req.quantity_change
                ),
                new_stock_quantity: product.stock_quantity,
            }));
        }

        // Update stock
        sqlx::query(
            "UPDATE products SET stock_quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(new_stock)
        .bind(&req.product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| Status::internal(format!("Commit error: {}", e)))?;

        Ok(Response::new(UpdateInventoryResponse {
            success: true,
            message: "Inventory updated successfully".to_string(),
            new_stock_quantity: new_stock,
        }))
    }
}
