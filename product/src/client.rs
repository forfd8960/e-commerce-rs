use proto::product::{
    AddProductRequest, CheckAvailabilityRequest, DeleteProductRequest, GetProductRequest,
    ListProductsRequest, UpdateInventoryRequest, UpdateProductRequest,
    product_service_client::ProductServiceClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ProductServiceClient::connect("http://127.0.0.1:50052").await?;

    println!("Connected to Product Service");
    println!("=============================\n");

    // Test 1: Add a new product
    println!("1. Testing Add Product");
    let add_request = AddProductRequest {
        name: "Laptop".to_string(),
        description: "High-performance laptop with 16GB RAM".to_string(),
        price: 1299.99,
        stock_quantity: 50,
        category: "Electronics".to_string(),
    };

    let add_response = client.add_product(add_request).await?;
    let add_result = add_response.into_inner();
    println!("Add Product Response:");
    println!("  Success: {}", add_result.success);
    println!("  Message: {}", add_result.message);
    println!("  Product ID: {}\n", add_result.product_id);

    let product_id = add_result.product_id.clone();

    // Test 2: Add another product
    println!("2. Testing Add Another Product");
    let add_request2 = AddProductRequest {
        name: "Wireless Mouse".to_string(),
        description: "Ergonomic wireless mouse with USB receiver".to_string(),
        price: 29.99,
        stock_quantity: 150,
        category: "Electronics".to_string(),
    };

    let add_response2 = client.add_product(add_request2).await?;
    let add_result2 = add_response2.into_inner();
    println!("Add Product Response:");
    println!("  Success: {}", add_result2.success);
    println!("  Message: {}", add_result2.message);
    println!("  Product ID: {}\n", add_result2.product_id);

    let product_id2 = add_result2.product_id.clone();

    // Test 3: Get product by ID
    println!("3. Testing Get Product");
    let get_request = GetProductRequest {
        product_id: product_id.clone(),
    };

    let get_response = client.get_product(get_request).await?;
    let get_result = get_response.into_inner();
    println!("Get Product Response:");
    println!("  Success: {}", get_result.success);
    println!("  Message: {}", get_result.message);
    if let Some(product) = &get_result.product {
        println!("  Product ID: {}", product.product_id);
        println!("  Name: {}", product.name);
        println!("  Description: {}", product.description);
        println!("  Price: ${:.2}", product.price);
        println!("  Stock: {}", product.stock_quantity);
        println!("  Category: {}\n", product.category);
    }

    // Test 4: List all products
    println!("4. Testing List Products");
    let list_request = ListProductsRequest {
        page: 1,
        page_size: 10,
        category: String::new(),
    };

    let list_response = client.list_products(list_request).await?;
    let list_result = list_response.into_inner();
    println!("List Products Response:");
    println!("  Success: {}", list_result.success);
    println!("  Message: {}", list_result.message);
    println!("  Total Count: {}", list_result.total_count);
    println!("  Products in this page:");
    for product in &list_result.products {
        println!(
            "    - {} (${:.2}) - Stock: {}",
            product.name, product.price, product.stock_quantity
        );
    }
    println!();

    // Test 5: List products by category
    println!("5. Testing List Products by Category");
    let list_by_category_request = ListProductsRequest {
        page: 1,
        page_size: 10,
        category: "Electronics".to_string(),
    };

    let list_by_category_response = client.list_products(list_by_category_request).await?;
    let list_by_category_result = list_by_category_response.into_inner();
    println!("List Products by Category Response:");
    println!("  Success: {}", list_by_category_result.success);
    println!(
        "  Total in Electronics: {}",
        list_by_category_result.total_count
    );
    println!("  Products:");
    for product in &list_by_category_result.products {
        println!("    - {}", product.name);
    }
    println!();

    // Test 6: Check availability
    println!("6. Testing Check Availability");
    let availability_request = CheckAvailabilityRequest {
        product_id: product_id.clone(),
        quantity: 25,
    };

    let availability_response = client.check_availability(availability_request).await?;
    let availability_result = availability_response.into_inner();
    println!("Check Availability Response:");
    println!("  Available: {}", availability_result.available);
    println!("  Message: {}", availability_result.message);
    println!("  Current Stock: {}\n", availability_result.current_stock);

    // Test 7: Update inventory (decrease stock)
    println!("7. Testing Update Inventory (Decrease)");
    let inventory_request = UpdateInventoryRequest {
        product_id: product_id.clone(),
        quantity_change: -10, // Decrease by 10
    };

    let inventory_response = client.update_inventory(inventory_request).await?;
    let inventory_result = inventory_response.into_inner();
    println!("Update Inventory Response:");
    println!("  Success: {}", inventory_result.success);
    println!("  Message: {}", inventory_result.message);
    println!(
        "  New Stock Quantity: {}\n",
        inventory_result.new_stock_quantity
    );

    // Test 8: Update inventory (increase stock)
    println!("8. Testing Update Inventory (Increase)");
    let inventory_request2 = UpdateInventoryRequest {
        product_id: product_id.clone(),
        quantity_change: 25, // Increase by 25
    };

    let inventory_response2 = client.update_inventory(inventory_request2).await?;
    let inventory_result2 = inventory_response2.into_inner();
    println!("Update Inventory Response:");
    println!("  Success: {}", inventory_result2.success);
    println!("  Message: {}", inventory_result2.message);
    println!(
        "  New Stock Quantity: {}\n",
        inventory_result2.new_stock_quantity
    );

    // Test 9: Update product
    println!("9. Testing Update Product");
    let update_request = UpdateProductRequest {
        product_id: product_id.clone(),
        name: "Gaming Laptop".to_string(),
        description: "High-performance gaming laptop with RTX GPU and 32GB RAM".to_string(),
        price: 1899.99,
        stock_quantity: 65,
        category: "Gaming".to_string(),
    };

    let update_response = client.update_product(update_request).await?;
    let update_result = update_response.into_inner();
    println!("Update Product Response:");
    println!("  Success: {}", update_result.success);
    println!("  Message: {}", update_result.message);
    if let Some(product) = &update_result.product {
        println!("  Updated Name: {}", product.name);
        println!("  Updated Price: ${:.2}", product.price);
        println!("  Updated Category: {}\n", product.category);
    }

    // Test 10: Check availability with insufficient stock
    println!("10. Testing Check Availability (Insufficient Stock)");
    let availability_request2 = CheckAvailabilityRequest {
        product_id: product_id.clone(),
        quantity: 1000,
    };

    let availability_response2 = client.check_availability(availability_request2).await?;
    let availability_result2 = availability_response2.into_inner();
    println!("Check Availability Response:");
    println!("  Available: {}", availability_result2.available);
    println!("  Message: {}", availability_result2.message);
    println!("  Current Stock: {}\n", availability_result2.current_stock);

    // Test 11: Delete product
    println!("11. Testing Delete Product");
    let delete_request = DeleteProductRequest {
        product_id: product_id2.clone(),
    };

    let delete_response = client.delete_product(delete_request).await?;
    let delete_result = delete_response.into_inner();
    println!("Delete Product Response:");
    println!("  Success: {}", delete_result.success);
    println!("  Message: {}\n", delete_result.message);

    // Test 12: Try to get deleted product
    println!("12. Testing Get Deleted Product");
    let get_deleted_request = GetProductRequest {
        product_id: product_id2.clone(),
    };

    let get_deleted_response = client.get_product(get_deleted_request).await?;
    let get_deleted_result = get_deleted_response.into_inner();
    println!("Get Deleted Product Response:");
    println!("  Success: {}", get_deleted_result.success);
    println!("  Message: {}\n", get_deleted_result.message);

    println!("=============================");
    println!("All tests completed!");

    Ok(())
}
