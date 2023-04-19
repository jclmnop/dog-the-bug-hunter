// endpoint-enumerator-interface.smithy

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "org.wasmcloud.interface.surrealdb", crate: "wasmcloud-interface-surrealdb" } ]

namespace org.wasmcloud.interface.surrealdb

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U32

/// Interact with a SurrealDB instance
@wasmbus(
  contractId: "wasmcloud:surrealdb",
  providerReceive: true,
)
service SurrealDb {
  version: "0.1",
  operations: [Query, SignUp, SignIn, Authenticate]
}

/// Send a query to the SurrealDb instance.
operation Query {
  input: QueryRequest,
  output: QueryResponses,
}

/// Sign a new user up to an existing scope. If the scope has not been set up
/// beforehand then this request will fail.
///
/// It's recommended to set up password hashing when defining the sign_up/sign_in
/// functions for a new scope:
/// ```
/// DEFINE SCOPE admin SESSION 1d
///        SIGNUP ( CREATE user SET user = $user, pass = crypto::argon2::generate($pass) )
///        SIGNIN ( SELECT * FROM user WHERE user = $user AND crypto::argon2::compare(pass, $pass));
/// ```
operation SignUp {
  input: Scope,
  output: SignUpResponse,
}

operation SignIn {
  input: Scope,
  output: SignInResponse,
}

operation Authenticate {
  input: String,
  output: AuthenticateResponse,
}

/// A query can be a combination of any
/// types of SurrealQL query: SELECT, CREATE, UPDATE, TRANSACTION, etc.
///
/// `queries` and `bindings` are both lists of lists of bytes. `bindings[i]`
/// contains the bindings for `statements[i]`. If `statements[i]` doesn't require
/// any bindings then `bindings[i]` should be an empty list.
///
/// Each query is executed in sequence.
structure QueryRequest {
  /// A list of SurrealQL statements, each represented as a string. If user input is being
  /// inserted into the statement, it is recommended to use binding variables
  /// and values to avoid SQL injection.
  @required
  queries: Queries,
  /// The binding values for each statement. Can either be key value pairs or a
  /// Struct, serialised into a string. If a query doesn't require any bindings
  /// then an empty list should be included.
  @required
  bindings: Bindings,
  /// The scope in which the query should be executed for. If no scope is
  /// provided then the default namespace and database will be used, with
  /// the Root scope configured for this database.
  ///
  /// If using a custom scope, perhaps for a user, then this scope must have
  /// been set up beforehand otherwise the query will fail. The query will
  /// also fail if a custom scope is provided but the authentication credentials
  /// are invalid.
  scope: Scope,
}

/// Result of querying a SurrealDb instance. If error value is present, then
/// the query failed.
structure QueryResponse {
  /// The data returend from each query, serialised into bytes. Will be empty if no data
  /// was retrieved.
  ///
  /// SurrealDB responses are usually of type `IndexMap<usize, surrealdb::Result<Vec<surrealdb::sql::Value>>>`,
  /// but here the Results are unwrapped and any errors will be included as a SurrealDbError in the QueryResponse.
  ///
  /// Each `Vec<u8>` can be deserialised directly into a `Vec<T>` where `T` is a struct with the required fields.
  @required
  response: ResponseData,
  /// Details of any error(s) that caused a query to fail. If this is empty,
  /// then all the queries succeeded.
  @required
  errors: SurrealDbErrors,
}

list ResponseData {
  member: Blob
}

/// List of responses for each query that was executed.
list QueryResponses {
  member: QueryResponse,
}

structure SignUpResponse {
  @required
  success: Boolean,
  @sensitive
  jwt: String,
  error: SurrealDbError,
}

structure SignInResponse {
  @required
  success: Boolean,
  @sensitive
  jwt: String,
  error: SurrealDbError,
}

structure AuthenticateResponse {
  @required
  success: Boolean,
  error: SurrealDbError,
}

/// The scope in which a query should be executed. All fields are optional,
/// but namespace and database must have defaults configured in the provider.
structure Scope {
  /// The namespace to be used, defaults to user configured default value.
  namespace: String,
  /// The database to be used, defaults to user configured default value.
  database: String,
  /// The scope to be used, defaults to Root.
  scopeName: String,
  /// Authentication paramaters, only required if scope is specified.
  @sensitive
  authParams: AuthParams,
  @sensitive
  jwt: String,
}

/// Either a username and password or a JWT
structure AuthParams {
  @sensitive @required
  username: String,
  @sensitive @required
  password: String,
}

/// List of bindings for a query. Value must be serialised.
list Bindings {
  member: String,
}

list Queries {
  member: String,
}

structure SurrealDbError {
  @required
  name: String,
  @required
  message: String,
}

list SurrealDbErrors {
  member: SurrealDbError,
}