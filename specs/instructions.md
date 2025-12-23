I am designing a e-commerce system, it has user, product, order, this 2 service.
the user service manages user information, the product service manages product information, the order service manages order information.

Each service is a separate microservice, they communicate with each other using gRPC.

## User Service
The user service provides APIs to register, login, verify(by token) and manage user profiles. It stores user data in a database.

## Product Service
The product service provides APIs to add, update, delete, and retrieve product information. It also manages product inventory. It stores product data in a database.

## Order Service
The order service provides APIs to create, update, cancel, and retrieve orders. It interacts with the user service to verify user information and with the product service to check product availability. It stores order data in a database.

## Database

- use postgres as database
- all service use the same database instance, but each service has its own schema to isolate data.
- use sqlx as the ORM layer.

## User Service Implement

- proto: proto/user.proto
- service name: UserService
- use tonic to implement gRPC server and client.
- implement user registration, login, token verification, and profile management.
- hash user passwords before storing them in the database.
- generate JWT tokens for user authentication.
- implement input validation and error handling.

## Product Service Implement
- proto: proto/product.proto
- service name: ProductService
- import product mod with: `use proto::product::{...}`
- use tonic to implement gRPC server and client.
- implement product addition, update, deletion, and retrieval.
- manage product inventory levels.
- implement input validation and error handling.


## Order Service Implement
- proto: proto/order.proto
- service name: OrderService
- import order service with: `use proto::order::{...}`
- use tonic to implement gRPC server and client.
- implement order creation, update, cancellation, and retrieval.