//! A mock fullnode for tests: the generated tonic service stubs, serving
//! canned objects, owned-object listings, dynamic fields and coin metadata
//! from memory on a local port.
//!
//! Executing a transaction records it as a [`Submission`] and answers with
//! the [`Execution`] the test set, filled in only as far as the request's
//! read mask asks — a field the caller forgot to request comes back missing,
//! as from a real node. A successful execution stores the objects it creates
//! and mutates, so reads after it see them.
//!
//! Compiled for `ptbs`'s own tests, and for other crates' tests behind the
//! `test-helpers` feature.

use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
};

use futures_util::Stream;
use iota_sdk_grpc_client::{GrpcClient, tonic};
use iota_sdk_grpc_types::{
    google::rpc::Status as RpcStatus,
    v1::{
        bcs::BcsData,
        coin::CoinMetadata,
        dynamic_field::{DynamicField, dynamic_field::DynamicFieldKind as ProtoKind},
        event::{Event as ProtoEvent, Events},
        ledger_service::{
            CheckpointData, GetCheckpointRequest, GetEpochRequest, GetEpochResponse,
            GetHealthRequest, GetHealthResponse, GetObjectsRequest, GetObjectsResponse,
            GetServiceInfoRequest, GetServiceInfoResponse, GetTransactionsRequest,
            GetTransactionsResponse, ObjectResult, StreamCheckpointsRequest,
            ledger_service_server::{LedgerService, LedgerServiceServer},
            object_result,
        },
        object::{Object as ProtoObject, Objects},
        state_service::{
            GetCoinInfoRequest, GetCoinInfoResponse, ListDynamicFieldsRequest,
            ListDynamicFieldsResponse, ListOwnedObjectsRequest, ListOwnedObjectsResponse,
            state_service_server::{StateService, StateServiceServer},
        },
        transaction::{
            ExecutedTransaction, Transaction as ProtoTransaction,
            TransactionEffects as ProtoEffects, TransactionEvents,
        },
        transaction_execution_service::{
            ExecuteTransactionItem, ExecuteTransactionResult, ExecuteTransactionsRequest,
            ExecuteTransactionsResponse, SimulateTransactionsRequest, SimulateTransactionsResponse,
            ViewFunctionCallsRequest, ViewFunctionCallsResponse, execute_transaction_result,
            transaction_execution_service_server::{
                TransactionExecutionService, TransactionExecutionServiceServer,
            },
        },
        versioned::{VersionedEvent, VersionedObject},
    },
};
use iota_sdk_types::{
    Address, ChangedObject, Event, ExecutionStatus, GasCostSummary, IdOperation, MoveObjectType,
    MoveStruct, Object, ObjectData, ObjectId, ObjectIn, ObjectOut, Owner, StructTag, Transaction,
    TransactionDigest, TransactionEffects, TransactionEffectsV1, TypeTag, UserSignature, Version,
};
use serde::Serialize;
use tonic::{Code, Request, Response, Status};

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

/// What the node does with the transactions it executes.
#[derive(Clone, Debug)]
pub struct Execution {
    pub status: ExecutionStatus,
    /// Objects the transaction creates.
    pub created: Vec<Object>,
    /// Objects it changes, returned among the output objects with the
    /// created ones.
    pub mutated: Vec<Object>,
    pub events: Vec<Event>,
}

impl Execution {
    /// A successful execution that creates `created` and emits `events`.
    pub fn success(created: Vec<Object>, events: Vec<Event>) -> Execution {
        return Execution {
            status: ExecutionStatus::Success,
            created,
            mutated: vec![],
            events,
        };
    }
}

/// A transaction the node was asked to execute, with the request's options.
#[derive(Clone, Debug)]
pub struct Submission {
    pub transaction: Transaction,
    pub signatures: Vec<UserSignature>,
    pub checkpoint_inclusion_timeout_ms: Option<u64>,
    pub read_mask: Vec<String>,
}

/// The node's contents and the failures it is told to produce.
#[derive(Default)]
struct NodeState {
    objects: HashMap<ObjectId, Object>,
    /// each owner's objects in the order the node lists them
    owned: HashMap<Address, Vec<ObjectId>>,
    fields: HashMap<ObjectId, Vec<DynamicField>>,
    coin_metadata: HashMap<String, CoinMetadata>,
    execution: Option<Execution>,
    submissions: Vec<Submission>,
    /// `get_objects` calls still to fail as Unavailable
    unavailable_reads: u32,
    get_objects_calls: u32,
    list_fields_calls: u32,
}

impl NodeState {
    fn insert(&mut self, object: Object) {
        if let Owner::Address(owner) = object.owner() {
            let listed = self.owned.entry(*owner).or_default();
            if !listed.contains(&object.id()) {
                listed.push(object.id());
            }
        }
        self.objects.insert(object.id(), object);
    }

    /// Record `submission` and answer it with the set [`Execution`].
    fn execute(&mut self, submission: Submission) -> ExecuteTransactionResult {
        let digest = submission.transaction.digest();
        let read_mask = submission.read_mask.clone();
        self.submissions.push(submission);

        let mut result = ExecuteTransactionResult::default();
        let Some(execution) = self.execution.clone() else {
            let status = rpc_status(Code::FailedPrecondition, "mock node: no execution set");
            result.result = Some(execute_transaction_result::Result::Error(status));
            return result;
        };
        if execution.status == ExecutionStatus::Success {
            for object in execution.created.iter().chain(&execution.mutated) {
                self.insert(object.clone());
            }
        }
        let executed = execution.executed(digest, &read_mask);
        result.result = Some(execute_transaction_result::Result::ExecutedTransaction(
            executed,
        ));
        return result;
    }
}

/// A fullnode serving from memory; clones share one state.
#[derive(Clone, Default)]
pub struct MockNode {
    state: Arc<Mutex<NodeState>>,
}

impl MockNode {
    /// Serve the node on a free local port and return a client for it.
    pub async fn start(&self) -> GrpcClient {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let incoming = futures_util::stream::unfold(listener, |listener| async move {
            let connection = listener.accept().await.map(|(stream, _)| stream);
            return Some((connection, listener));
        });
        let server = tonic::transport::Server::builder()
            .add_service(LedgerServiceServer::new(self.clone()))
            .add_service(StateServiceServer::new(self.clone()))
            .add_service(TransactionExecutionServiceServer::new(self.clone()))
            .serve_with_incoming(incoming);
        tokio::spawn(server);
        return GrpcClient::new(format!("http://{address}")).unwrap();
    }

    fn state(&self) -> MutexGuard<'_, NodeState> {
        return self.state.lock().unwrap();
    }

    /// Store `object`, listed under its owner when an address owns it.
    pub fn insert(&self, object: Object) {
        self.state().insert(object);
    }

    pub fn set_fields(&self, parent: ObjectId, fields: Vec<DynamicField>) {
        self.state().fields.insert(parent, fields);
    }

    pub fn set_coin_metadata(&self, coin_type: &str, metadata: CoinMetadata) {
        self.state()
            .coin_metadata
            .insert(coin_type.to_string(), metadata);
    }

    /// Answer every transaction executed from now on with `execution`.
    pub fn set_execution(&self, execution: Execution) {
        self.state().execution = Some(execution);
    }

    /// The transactions executed so far, in order.
    pub fn submissions(&self) -> Vec<Submission> {
        return self.state().submissions.clone();
    }

    /// Fail the next `calls` object reads as if the node were unreachable.
    pub fn fail_reads(&self, calls: u32) {
        self.state().unavailable_reads = calls;
    }

    pub fn get_objects_calls(&self) -> u32 {
        return self.state().get_objects_calls;
    }

    pub fn list_fields_calls(&self) -> u32 {
        return self.state().list_fields_calls;
    }
}

impl Execution {
    fn effects(&self, digest: TransactionDigest) -> TransactionEffects {
        let changed_objects = self
            .created
            .iter()
            .map(|object| ChangedObject {
                object_id: object.id(),
                input_state: ObjectIn::Missing,
                output_state: ObjectOut::ObjectWrite {
                    digest: object.object_ref().digest,
                    owner: *object.owner(),
                },
                id_operation: IdOperation::Created,
            })
            .collect();
        return TransactionEffects::V1(Box::new(TransactionEffectsV1 {
            status: self.status.clone(),
            epoch: 0,
            gas_cost_summary: GasCostSummary::default(),
            transaction_digest: digest,
            gas_object_index: None,
            events_digest: None,
            dependencies: vec![],
            lamport_version: Version::from(2u64),
            changed_objects,
            unchanged_shared_objects: vec![],
            auxiliary_data_digest: None,
        }));
    }

    /// The executed transaction as the node returns it: only what
    /// `read_mask` asks for, and no events field when there are none.
    fn executed(&self, digest: TransactionDigest, read_mask: &[String]) -> ExecutedTransaction {
        let mut executed = ExecutedTransaction::default();
        if read_mask.requests("transaction.digest") {
            let mut transaction = ProtoTransaction::default();
            transaction.digest = Some(digest.into());
            executed.transaction = Some(transaction);
        }
        if read_mask.requests("effects.bcs") {
            let mut effects = ProtoEffects::default();
            effects.bcs = Some(BcsData::serialize(&self.effects(digest)).unwrap());
            executed.effects = Some(effects);
        }
        if read_mask.requests("events.events.bcs") && !self.events.is_empty() {
            executed.events = Some(transaction_events(&self.events));
        }
        if read_mask.requests("output_objects.bcs") {
            let mut objects = Objects::default();
            objects.objects = self
                .created
                .iter()
                .chain(&self.mutated)
                .map(proto_object)
                .collect();
            executed.output_objects = Some(objects);
        }
        return executed;
    }
}

fn transaction_events(events: &[Event]) -> TransactionEvents {
    let mut listed = Events::default();
    listed.events = events
        .iter()
        .map(|event| {
            let mut proto = ProtoEvent::default();
            proto.bcs = Some(BcsData::serialize(&VersionedEvent::V1(event.clone())).unwrap());
            return proto;
        })
        .collect();
    let mut transaction_events = TransactionEvents::default();
    transaction_events.events = Some(listed);
    return transaction_events;
}

trait ReadMask {
    /// Whether the mask covers `path`: it names the path or a parent of it.
    fn requests(&self, path: &str) -> bool;
}

impl ReadMask for [String] {
    fn requests(&self, path: &str) -> bool {
        return self.iter().any(|requested| {
            return path == requested
                || path
                    .strip_prefix(requested.as_str())
                    .is_some_and(|rest| rest.starts_with('.'));
        });
    }
}

impl Submission {
    fn decode(
        item: &ExecuteTransactionItem,
        checkpoint_inclusion_timeout_ms: Option<u64>,
        read_mask: &[String],
    ) -> Result<Submission, Status> {
        let invalid = |e: &dyn std::fmt::Display| Status::invalid_argument(e.to_string());
        let transaction = item
            .transaction
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("no transaction"))?
            .transaction()
            .map_err(|e| invalid(&e))?;
        let signatures = item
            .signatures
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("no signatures"))?;
        return Ok(Submission {
            transaction,
            signatures: Vec::try_from(signatures).map_err(|e| invalid(&e))?,
            checkpoint_inclusion_timeout_ms,
            read_mask: read_mask.to_vec(),
        });
    }
}

/// A Move object of `type_` whose contents are `value` as BCS. The first
/// field of `value` must be the object's id, as on chain.
pub fn move_object(type_: StructTag, owner: Owner, version: u64, value: &impl Serialize) -> Object {
    return move_object_with_contents(type_, owner, version, bcs::to_bytes(value).unwrap());
}

fn move_object_with_contents(
    type_: StructTag,
    owner: Owner,
    version: u64,
    contents: Vec<u8>,
) -> Object {
    let data =
        MoveStruct::new(MoveObjectType::new(type_), Version::from(version), contents).unwrap();
    return Object::new(ObjectData::Struct(data), owner, TransactionDigest::ZERO, 0);
}

/// A coin of `coin_type`: `Coin { id, balance }`.
pub fn coin(coin_type: TypeTag, owner: Address, id: ObjectId, balance: u64) -> Object {
    return move_object(
        StructTag::new_coin(coin_type),
        Owner::Address(owner),
        1,
        &(id, balance),
    );
}

/// A gas coin: `Coin { id, balance }`.
pub fn gas_coin(owner: Address, id: ObjectId, balance: u64) -> Object {
    return coin(TypeTag::from(StructTag::new_gas()), owner, id, balance);
}

/// The `0x2::dynamic_field::Field<K, V>` object holding `parent`'s field
/// `name`, at the ID the chain derives for it: `Field { id, name, value }`.
pub fn field_object(
    parent: ObjectId,
    name_type: TypeTag,
    name: &impl Serialize,
    value_type: TypeTag,
    value: &impl Serialize,
) -> Object {
    let name = bcs::to_bytes(name).unwrap();
    let field_id = parent.derive_dynamic_child_id(&name_type, &name);
    let mut contents = bcs::to_bytes(&field_id).unwrap();
    contents.extend(name);
    contents.extend(bcs::to_bytes(value).unwrap());
    return move_object_with_contents(
        StructTag::new_dynamic_field(name_type, value_type),
        Owner::Object(parent),
        1,
        contents,
    );
}

/// `parent`'s field `name` as the listing returns it.
pub fn listed_field(
    parent: ObjectId,
    kind: ProtoKind,
    name_type: TypeTag,
    name: &impl Serialize,
    value_type: TypeTag,
    value: &impl Serialize,
) -> DynamicField {
    let field_object = field_object(parent, name_type, name, value_type.clone(), value);
    let field_id = field_object.id();
    let mut field = DynamicField::default();
    field.kind = Some(kind as i32);
    field.field_id = Some(field_id.into());
    field.field_object = Some(proto_object(&field_object));
    field.name = Some(BcsData::from(bcs::to_bytes(name).unwrap()));
    field.value_type = Some(value_type.to_string());
    return field;
}

fn proto_object(object: &Object) -> ProtoObject {
    let mut proto = ProtoObject::default();
    proto.reference = Some(object.object_ref().into());
    proto.bcs = Some(BcsData::serialize(&VersionedObject::V1(object.clone())).unwrap());
    return proto;
}

fn rpc_status(code: Code, message: impl Into<String>) -> RpcStatus {
    let mut status = RpcStatus::default();
    status.code = code as i32;
    status.message = message.into();
    return status;
}

#[tonic::async_trait]
impl LedgerService for MockNode {
    type GetObjectsStream = BoxStream<GetObjectsResponse>;
    type GetTransactionsStream = BoxStream<GetTransactionsResponse>;
    type GetCheckpointStream = BoxStream<CheckpointData>;
    type StreamCheckpointsStream = BoxStream<CheckpointData>;

    async fn get_objects(
        &self,
        request: Request<GetObjectsRequest>,
    ) -> Result<Response<Self::GetObjectsStream>, Status> {
        let mut state = self.state();
        state.get_objects_calls += 1;
        if state.unavailable_reads > 0 {
            state.unavailable_reads -= 1;
            return Err(Status::unavailable("node down"));
        }

        let requests = request.into_inner().requests.unwrap_or_default().requests;
        let objects = requests
            .iter()
            .map(|requested| {
                let id = requested
                    .object_ref
                    .as_ref()
                    .and_then(|r| r.object_id.as_ref())
                    .map(|id| id.object_id().unwrap())
                    .unwrap();
                let mut result = ObjectResult::default();
                result.result = Some(match state.objects.get(&id) {
                    Some(object) => object_result::Result::Object(proto_object(object)),
                    None => object_result::Result::Error(rpc_status(
                        Code::NotFound,
                        format!("object {id} not found"),
                    )),
                });
                return result;
            })
            .collect();
        let mut response = GetObjectsResponse::default();
        response.objects = objects;
        response.has_next = false;
        let stream = futures_util::stream::iter([Ok(response)]);
        return Ok(Response::new(Box::pin(stream)));
    }

    async fn get_health(
        &self,
        _: Request<GetHealthRequest>,
    ) -> Result<Response<GetHealthResponse>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn get_service_info(
        &self,
        _: Request<GetServiceInfoRequest>,
    ) -> Result<Response<GetServiceInfoResponse>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn get_transactions(
        &self,
        _: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn get_checkpoint(
        &self,
        _: Request<GetCheckpointRequest>,
    ) -> Result<Response<Self::GetCheckpointStream>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn stream_checkpoints(
        &self,
        _: Request<StreamCheckpointsRequest>,
    ) -> Result<Response<Self::StreamCheckpointsStream>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn get_epoch(
        &self,
        _: Request<GetEpochRequest>,
    ) -> Result<Response<GetEpochResponse>, Status> {
        return Err(Status::unimplemented("mock node"));
    }
}

#[tonic::async_trait]
impl StateService for MockNode {
    async fn list_owned_objects(
        &self,
        request: Request<ListOwnedObjectsRequest>,
    ) -> Result<Response<ListOwnedObjectsResponse>, Status> {
        let request = request.into_inner();
        let owner = request.owner.as_ref().unwrap().address().unwrap();
        let state = self.state();
        let objects = state
            .owned
            .get(&owner)
            .into_iter()
            .flatten()
            .map(|id| &state.objects[id])
            .filter(|object| {
                let type_ = object.struct_tag().map(|tag| tag.to_string());
                return request.object_type.is_none() || type_ == request.object_type;
            })
            .map(proto_object)
            .collect();
        let mut response = ListOwnedObjectsResponse::default();
        response.objects = objects;
        return Ok(Response::new(response));
    }

    async fn list_dynamic_fields(
        &self,
        request: Request<ListDynamicFieldsRequest>,
    ) -> Result<Response<ListDynamicFieldsResponse>, Status> {
        let parent = request.into_inner().parent.unwrap().object_id().unwrap();
        let mut state = self.state();
        state.list_fields_calls += 1;
        let mut response = ListDynamicFieldsResponse::default();
        response.dynamic_fields = state.fields.get(&parent).cloned().unwrap_or_default();
        return Ok(Response::new(response));
    }

    async fn get_coin_info(
        &self,
        request: Request<GetCoinInfoRequest>,
    ) -> Result<Response<GetCoinInfoResponse>, Status> {
        let coin_type = request.into_inner().coin_type.unwrap_or_default();
        let Some(metadata) = self.state().coin_metadata.get(&coin_type).cloned() else {
            return Err(Status::not_found(format!("no coin {coin_type}")));
        };
        let mut response = GetCoinInfoResponse::default();
        response.coin_type = Some(coin_type);
        response.metadata = Some(metadata);
        return Ok(Response::new(response));
    }
}

#[tonic::async_trait]
impl TransactionExecutionService for MockNode {
    async fn execute_transactions(
        &self,
        request: Request<ExecuteTransactionsRequest>,
    ) -> Result<Response<ExecuteTransactionsResponse>, Status> {
        let request = request.into_inner();
        let read_mask = request.read_mask.map(|mask| mask.paths).unwrap_or_default();
        let submissions = request
            .transactions
            .iter()
            .map(|item| {
                return Submission::decode(
                    item,
                    request.checkpoint_inclusion_timeout_ms,
                    &read_mask,
                );
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let mut state = self.state();
        let mut response = ExecuteTransactionsResponse::default();
        response.transaction_results = submissions
            .into_iter()
            .map(|submission| state.execute(submission))
            .collect();
        return Ok(Response::new(response));
    }

    async fn simulate_transactions(
        &self,
        _: Request<SimulateTransactionsRequest>,
    ) -> Result<Response<SimulateTransactionsResponse>, Status> {
        return Err(Status::unimplemented("mock node"));
    }

    async fn view_function_calls(
        &self,
        _: Request<ViewFunctionCallsRequest>,
    ) -> Result<Response<ViewFunctionCallsResponse>, Status> {
        return Err(Status::unimplemented("mock node"));
    }
}
