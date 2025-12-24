use proto::order::{
    CancelOrderRequest, CreateOrderRequest, GetOrderRequest, GetOrdersByUserRequest,
    ListOrdersRequest, OrderItem, OrderStatus, UpdateOrderRequest,
    order_service_client::OrderServiceClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderServiceClient::connect("http://127.0.0.1:50053").await?;

    println!("Connected to Order Service");
    println!("===========================\n");

    // Note: You'll need to have created a user and products first
    // For this example, we'll use placeholder IDs
    let user_id = "test-user-id".to_string();
    let product_id_1 = "test-product-id-1".to_string();
    let product_id_2 = "test-product-id-2".to_string();

    println!("Note: Make sure you have created a user and products first!");
    println!("User ID: {}", user_id);
    println!("Product IDs: {}, {}\n", product_id_1, product_id_2);

    // Test 1: Create a new order
    println!("1. Testing Create Order");
    let create_request = CreateOrderRequest {
        user_id: user_id.clone(),
        items: vec![
            OrderItem {
                product_id: product_id_1.clone(),
                product_name: String::new(),
                quantity: 2,
                unit_price: 0.0, // Will be set by server
                subtotal: 0.0,   // Will be calculated by server
            },
            OrderItem {
                product_id: product_id_2.clone(),
                product_name: String::new(),
                quantity: 1,
                unit_price: 0.0,
                subtotal: 0.0,
            },
        ],
        shipping_address: "123 Main St, City, State 12345".to_string(),
    };

    let create_response = client.create_order(create_request).await?;
    let create_result = create_response.into_inner();
    println!("Create Order Response:");
    println!("  Success: {}", create_result.success);
    println!("  Message: {}", create_result.message);
    println!("  Order ID: {}", create_result.order_id);
    if let Some(order) = &create_result.order {
        println!("  Total Amount: ${:.2}", order.total_amount);
        println!("  Status: {:?}", OrderStatus::try_from(order.status));
        println!("  Items count: {}", order.items.len());
        for (i, item) in order.items.iter().enumerate() {
            println!(
                "    Item {}: Product {}, Qty: {}, Price: ${:.2}, Subtotal: ${:.2}",
                i + 1,
                item.product_id,
                item.quantity,
                item.unit_price,
                item.subtotal
            );
        }
    }
    println!();

    let order_id = create_result.order_id.clone();

    // Test 2: Get order by ID
    println!("2. Testing Get Order");
    let get_request = GetOrderRequest {
        order_id: order_id.clone(),
    };

    let get_response = client.get_order(get_request).await?;
    let get_result = get_response.into_inner();
    println!("Get Order Response:");
    println!("  Success: {}", get_result.success);
    println!("  Message: {}", get_result.message);
    if let Some(order) = &get_result.order {
        println!("  Order ID: {}", order.order_id);
        println!("  User ID: {}", order.user_id);
        println!("  Total: ${:.2}", order.total_amount);
        println!("  Status: {:?}", OrderStatus::try_from(order.status));
        println!("  Shipping Address: {}", order.shipping_address);
    }
    println!();

    // Test 3: List all orders
    println!("3. Testing List Orders");
    let list_request = ListOrdersRequest {
        page: 1,
        page_size: 10,
        status: 0, // All statuses
    };

    let list_response = client.list_orders(list_request).await?;
    let list_result = list_response.into_inner();
    println!("List Orders Response:");
    println!("  Success: {}", list_result.success);
    println!("  Message: {}", list_result.message);
    println!("  Total Count: {}", list_result.total_count);
    println!("  Orders in this page:");
    for order in &list_result.orders {
        println!(
            "    - Order {}: ${:.2} - {:?}",
            order.order_id,
            order.total_amount,
            OrderStatus::try_from(order.status)
        );
    }
    println!();

    // Test 4: Get orders by user
    println!("4. Testing Get Orders by User");
    let user_orders_request = GetOrdersByUserRequest {
        user_id: user_id.clone(),
        page: 1,
        page_size: 10,
    };

    let user_orders_response = client.get_orders_by_user(user_orders_request).await?;
    let user_orders_result = user_orders_response.into_inner();
    println!("Get Orders by User Response:");
    println!("  Success: {}", user_orders_result.success);
    println!("  Message: {}", user_orders_result.message);
    println!("  Total Count: {}", user_orders_result.total_count);
    println!("  User's orders:");
    for order in &user_orders_result.orders {
        println!("    - Order {}: ${:.2}", order.order_id, order.total_amount);
    }
    println!();

    // Test 5: Update order status
    println!("5. Testing Update Order");
    let update_request = UpdateOrderRequest {
        order_id: order_id.clone(),
        status: OrderStatus::Confirmed as i32,
        shipping_address: "456 Updated St, New City, State 54321".to_string(),
    };

    let update_response = client.update_order(update_request).await?;
    let update_result = update_response.into_inner();
    println!("Update Order Response:");
    println!("  Success: {}", update_result.success);
    println!("  Message: {}", update_result.message);
    if let Some(order) = &update_result.order {
        println!(
            "  Updated Status: {:?}",
            OrderStatus::try_from(order.status)
        );
        println!("  Updated Address: {}", order.shipping_address);
    }
    println!();

    // Test 6: Update to processing
    println!("6. Testing Update to Processing");
    let update_request2 = UpdateOrderRequest {
        order_id: order_id.clone(),
        status: OrderStatus::Processing as i32,
        shipping_address: String::new(), // Keep existing
    };

    let update_response2 = client.update_order(update_request2).await?;
    let update_result2 = update_response2.into_inner();
    println!("Update Order Response:");
    println!("  Success: {}", update_result2.success);
    if let Some(order) = &update_result2.order {
        println!("  Status: {:?}", OrderStatus::try_from(order.status));
    }
    println!();

    // Test 7: List orders by status
    println!("7. Testing List Orders by Status (Processing)");
    let list_by_status_request = ListOrdersRequest {
        page: 1,
        page_size: 10,
        status: OrderStatus::Processing as i32,
    };

    let list_by_status_response = client.list_orders(list_by_status_request).await?;
    let list_by_status_result = list_by_status_response.into_inner();
    println!("List Orders by Status Response:");
    println!(
        "  Total in Processing: {}",
        list_by_status_result.total_count
    );
    println!("  Processing orders:");
    for order in &list_by_status_result.orders {
        println!("    - Order {}: ${:.2}", order.order_id, order.total_amount);
    }
    println!();

    // Test 8: Create another order to cancel
    println!("8. Testing Create Another Order (to cancel)");
    let create_request2 = CreateOrderRequest {
        user_id: user_id.clone(),
        items: vec![OrderItem {
            product_id: product_id_1.clone(),
            product_name: String::new(),
            quantity: 1,
            unit_price: 0.0,
            subtotal: 0.0,
        }],
        shipping_address: "789 Test Ave, Test City".to_string(),
    };

    let create_response2 = client.create_order(create_request2).await?;
    let create_result2 = create_response2.into_inner();
    println!("Create Order Response:");
    println!("  Success: {}", create_result2.success);
    println!("  Order ID: {}", create_result2.order_id);
    let order_id_to_cancel = create_result2.order_id.clone();
    println!();

    // Test 9: Cancel order
    println!("9. Testing Cancel Order");
    let cancel_request = CancelOrderRequest {
        order_id: order_id_to_cancel.clone(),
        user_id: user_id.clone(),
    };

    let cancel_response = client.cancel_order(cancel_request).await?;
    let cancel_result = cancel_response.into_inner();
    println!("Cancel Order Response:");
    println!("  Success: {}", cancel_result.success);
    println!("  Message: {}", cancel_result.message);
    println!();

    // Test 10: Verify cancelled order
    println!("10. Testing Get Cancelled Order");
    let get_cancelled_request = GetOrderRequest {
        order_id: order_id_to_cancel.clone(),
    };

    let get_cancelled_response = client.get_order(get_cancelled_request).await?;
    let get_cancelled_result = get_cancelled_response.into_inner();
    println!("Get Cancelled Order Response:");
    println!("  Success: {}", get_cancelled_result.success);
    if let Some(order) = &get_cancelled_result.order {
        println!("  Status: {:?}", OrderStatus::try_from(order.status));
    }
    println!();

    // Test 11: Try to cancel already cancelled order
    println!("11. Testing Cancel Already Cancelled Order");
    let cancel_request2 = CancelOrderRequest {
        order_id: order_id_to_cancel.clone(),
        user_id: user_id.clone(),
    };

    let cancel_response2 = client.cancel_order(cancel_request2).await?;
    let cancel_result2 = cancel_response2.into_inner();
    println!("Cancel Already Cancelled Order Response:");
    println!("  Success: {}", cancel_result2.success);
    println!("  Message: {}", cancel_result2.message);
    println!();

    println!("===========================");
    println!("All tests completed!");

    Ok(())
}
